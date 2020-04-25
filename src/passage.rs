extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use std::time::Duration;

use actix::prelude::*;
use chrono::Local;
use chrono::NaiveTime;

#[path = "route_fragment_registry.rs"]
mod route_fragment_registry;

#[derive(Debug, Serialize, Deserialize)]
pub struct PassageWelcome {
    actual: Vec<PassageActual>,
    #[serde(rename = "directionText")]
    direction_text: String,
    old: Vec<PassageActual>,
    #[serde(rename = "routeName")]
    route_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassageActual {
    #[serde(rename = "actualTime")]
    actual_time: Option<String>,
    planned_time: Option<String>,
    status: PassageStatus,
    stop: PassageStop,
    stop_seq_num: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassageStop {
    id: String,
    name: String,
    #[serde(rename = "shortName")]
    short_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PassageStatus {
    #[serde(rename = "DEPARTED")]
    Departed,
    #[serde(rename = "PREDICTED")]
    Predicted,
    #[serde(rename = "PLANNED")]
    Planned,
    #[serde(rename = "STOPPING")]
    Stopping,
}

async fn fetch_tram_passage(s: String) -> reqwest::Result<PassageWelcome> {
    println!("fetch");
    let fs = format!("https://mpk.jacekk.net/proxy_tram.php/services/tripInfo/tripPassages?tripId={}&mode=departure", s);
    let url = reqwest::Url::parse(&fs).unwrap();

    reqwest::get(url).await?.json::<PassageWelcome>().await
}

#[derive(Debug)]
pub struct TripMeta {
    pub route_name: String,
    pub direction_text: String,
}

#[derive(Debug)]
pub struct Trip {
    id: String,
    trip_meta: Option<TripMeta>,
    stop_seq: Option<u32>,
    next_stop: Option<String>,
    last_progress_time: Option<std::time::Instant>,
    update_time: Option<std::time::Instant>,
}

impl Trip {
    pub fn new(id: String) -> Trip {
        Trip {
            id: id,
            trip_meta: None,
            stop_seq: None,
            next_stop: None,
            last_progress_time: None,
            update_time: None,
        }
    }
}

impl Actor for Trip {
    type Context = Context<Trip>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(SelfFetchUpdateRequest);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DirectPassageSync {
    passage: PassageWelcome,
}

#[derive(Message)]
#[rtype(result = "()")]
struct SelfFetchUpdateRequest;

impl Handler<SelfFetchUpdateRequest> for Trip {
    type Result = ();

    fn handle(&mut self, _msg: SelfFetchUpdateRequest, ctx: &mut Context<Self>) {
        let z = String::from(&self.id);
        let x = actix::fut::wrap_future::<_, Self>(fetch_tram_passage(z));
        ctx.wait(x.map(|_result, _actor, _ctx| {
            _ctx.notify(DirectPassageSync {
                passage: _result.unwrap(),
            });
        }));
        ctx.notify_later(SelfFetchUpdateRequest, Duration::from_secs(60));
    }
}

impl Handler<DirectPassageSync> for Trip {
    type Result = ();

    fn handle(&mut self, _msg: DirectPassageSync, _ctx: &mut Context<Self>) {
        self.update_time = Some(std::time::Instant::now());

        if self.trip_meta.is_none() {
            self.trip_meta = Some(TripMeta {
                route_name: _msg.passage.route_name,
                direction_text: _msg.passage.direction_text,
            });
        }

        if let Some(actual_passage) = _msg.passage.actual.first() {
            let new_stop_seq = actual_passage.stop_seq_num.parse::<u32>().unwrap();

            if Some(new_stop_seq) != self.stop_seq {
                let new_stop = &actual_passage.stop.name;

                println!(
                    "{} {:?} made progress: {:?} -> {:?}",
                    &self.id, &self.trip_meta, self.next_stop, new_stop
                );

                self.last_progress_time = self.update_time;
                self.stop_seq = Some(new_stop_seq);
                self.next_stop = Some(String::from(new_stop));

                let rr = route_fragment_registry::RouteFragmentRegistry::from_registry();

                let f = rr
                    .send(route_fragment_registry::GetRouteFragment::new(
                        String::from(&actual_passage.stop.id),
                    ))
                    .into_actor(self)
                    .map(|result, actor, _ctx| {
                        result.unwrap().do_send(
                            route_fragment_registry::route_fragment::FragmentEntryEvent {
                                trip_id: String::from(&actor.id),
                                instant: std::time::Instant::now(),
                                time: Local::now().time(),
                            },
                        )
                    });

                _ctx.spawn(f);
            }

            if let Some(old_stop) = _msg.passage.old.last() {
                let ttime = old_stop
                    .actual_time
                    .as_ref()
                    .map(|x| NaiveTime::parse_from_str(&x, "%H:%M").unwrap())
                    .unwrap();

                println!("Old passage: {}", old_stop.stop.id);
                let z = route_fragment_registry::RouteFragmentRegistry::from_registry()
                    .send(route_fragment_registry::GetRouteFragment::new(
                        String::from(&old_stop.stop.id),
                    ))
                    .into_actor(self)
                    .map(move |result, actor, _ctx| {
                        result.unwrap().do_send(
                            route_fragment_registry::route_fragment::FragmentLeaveEvent {
                                trip_id: String::from(&actor.id),
                                instant: std::time::Instant::now(),
                                time: ttime,
                            },
                        )
                    });

                _ctx.spawn(z);
            }
        }

        println!("{:?}", self);
    }
}
