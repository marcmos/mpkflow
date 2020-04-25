extern crate serde;
extern crate serde_json;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::time::Duration;

use actix::prelude::*;
use actix_files::NamedFile;

use chrono::Local;
use futures::prelude::*;

mod passage;
mod route_fragment;
mod route_fragment_registry;
mod trip_registry;

use actix_web::{
    get, web, web::Data, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};

use tokio::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Welcome {
    actual: Vec<Actual>,
    directions: Vec<Option<serde_json::Value>>,
    #[serde(rename = "firstPassageTime")]
    first_passage_time: i64,
    #[serde(rename = "generalAlerts")]
    general_alerts: Vec<Option<serde_json::Value>>,
    #[serde(rename = "lastPassageTime")]
    last_passage_time: i64,
    old: Vec<Actual>,
    routes: Vec<Route>,
    #[serde(rename = "stopName")]
    stop_name: String,
    #[serde(rename = "stopShortName")]
    stop_short_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Actual {
    #[serde(rename = "actualRelativeTime")]
    actual_relative_time: i32,
    #[serde(rename = "actualTime")]
    actual_time: Option<String>,
    direction: String,
    #[serde(rename = "mixedTime")]
    mixed_time: String,
    passageid: String,
    #[serde(rename = "patternText")]
    pattern_text: String,
    #[serde(rename = "plannedTime")]
    planned_time: String,
    #[serde(rename = "routeId")]
    route_id: String,
    status: String,
    #[serde(rename = "tripId")]
    trip_id: String,
    #[serde(rename = "vehicleId")]
    vehicle_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Route {
    alerts: Vec<Option<serde_json::Value>>,
    authority: String,
    directions: Vec<String>,
    id: String,
    name: String,
    #[serde(rename = "routeType")]
    route_type: String,
    #[serde(rename = "shortName")]
    short_name: String,
}

async fn fetch_stop_passage(s: &str) -> reqwest::Result<Welcome> {
    println!("fetch");
    let fs = format!("https://mpk.jacekk.net/proxy_tram.php/services/passageInfo/stopPassages/stopPoint?stopPoint={}&mode=departure", s);
    let url = reqwest::Url::parse(&fs).unwrap();

    reqwest::get(url).await?.json::<Welcome>().await
}

#[derive(Debug)]
struct StopState {
    stop_id: &'static str,
    name: Option<String>,
    last_check: std::time::Instant,
    last_reparture_diff: Option<i32>,
    prev_stop: Option<String>,
}

impl Actor for StopState {
    type Context = Context<StopState>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct UpdateRequest {}

#[derive(Message)]
#[rtype(result = "()")]
struct PrintState {}

impl Handler<UpdateRequest> for StopState {
    type Result = ();

    fn handle(&mut self, _msg: UpdateRequest, ctx: &mut Context<Self>) {
        let x = actix::fut::wrap_future::<_, Self>(fetch_stop_passage(&self.stop_id));

        ctx.wait(x.map(|_result, actor, _ctx| {
            // println!("{:#?}", result);
            actor.last_reparture_diff = _result
                .as_ref()
                .unwrap()
                .old
                .first()
                .map(|x| x.actual_relative_time);

            let actual = _result.as_ref().unwrap().actual.first();

            let wait = actual.map(|x| x.actual_relative_time);

            actor.last_reparture_diff = _result
                .as_ref()
                .unwrap()
                .old
                .first()
                .map(|x| x.actual_relative_time);

            let z = match wait {
                Some(x) if x > 30 => x,
                _ => 120,
            };

            let update_reason = if let Some(act) = actual {
                Some(String::from(format!(
                    "{} {} {}",
                    act.pattern_text, act.direction, act.trip_id
                )))
            } else {
                None
            };

            println!(
                "{:?} update in {} due to {:?}",
                actor.name.as_ref().unwrap_or(&actor.stop_id.to_string()),
                z as u64,
                update_reason,
            );
            _ctx.notify_later(UpdateRequest {}, Duration::from_secs(z as u64));

            actor.name = Some(String::from(&_result.as_ref().unwrap().stop_name));

            let l = route_fragment_registry::RouteFragmentRegistry::from_registry()
                .send(route_fragment_registry::GetRouteFragment::new(
                    String::from(&_result.as_ref().unwrap().stop_short_name),
                ))
                .into_actor(actor);

            let stop_name = std::rc::Rc::new(String::from(&_result.as_ref().unwrap().stop_name));
            let s1 = stop_name.clone();

            let olde = std::rc::Rc::new(_result.unwrap().old);
            let xd = olde.clone();
            let xd2 = olde.clone();

            _ctx.wait(l.map(move |_rresult, _actor, _cctx| {
                let fragment = _rresult.unwrap();

                fragment.do_send(
                    route_fragment_registry::route_fragment::RouteFragment::update_start(&*s1),
                );

                xd.iter().for_each(|x| {
                    let lol = x.actual_relative_time.into();
                    let drift = chrono::Duration::seconds(lol);
                    let ttime = Local::now().time() + drift;

                    fragment.do_send(
                        route_fragment_registry::route_fragment::FragmentEntryEvent {
                            trip_id: String::from(&x.trip_id),
                            instant: std::time::Instant::now(),
                            time: ttime,
                        },
                    );
                });
            }));

            if let Some(stop) = &actor.prev_stop {
                let x = route_fragment_registry::RouteFragmentRegistry::from_registry()
                    .send(route_fragment_registry::GetRouteFragment::new(
                        String::from(stop),
                    ))
                    .into_actor(actor)
                    .map(move |rr, _, _| {
                        let fragment = rr.unwrap();

                        fragment.do_send(
                            route_fragment_registry::route_fragment::RouteFragment::update_stop(
                                &*stop_name.clone(),
                            ),
                        );

                        xd2.iter().for_each(|x| {
                            let ttime = Local::now().time()
                                + chrono::Duration::seconds(x.actual_relative_time.into());

                            fragment.do_send(
                                route_fragment_registry::route_fragment::FragmentLeaveEvent {
                                    trip_id: String::from(&x.trip_id),
                                    instant: std::time::Instant::now(),
                                    time: ttime,
                                },
                            );
                        });
                    });

                _ctx.wait(x);
            }
        }));
    }
}

impl Handler<PrintState> for StopState {
    type Result = ();

    fn handle(&mut self, _msg: PrintState, _ctx: &mut Context<Self>) {
        if let Some(n) = &self.name {
            println!("{}", n);
        }
        println!("{}", self.last_check.elapsed().as_secs());
        println!("{:#?}", self.last_reparture_diff);
    }
}

const STOP_IDS: [&str; 38] = [
    // Mogilskie → Czyżyny
    "12529", "12919", "13019", "304019", "281119", "11319", "11219", "40719",
    // Czyżyny → Mogilskie
    "40829", "40729", "11229", "11329", "281129", "304029", "13029", "12929",
    // Grzegórzeckie → Huta
    "36519", "285919", "36719", "36819", "36919", "37019", "303319", "287119", "93019", "304119",
    "40919", //
    // Huta → Grzegórzeckie
    "40849", "40929", "304129", "93029", "287129", "303329", "37029", "36929", "36829", "36729",
    "285929", //
];

async fn handle_frag_stat(
    _: HttpRequest,
    state: Data<Addr<route_fragment_registry::RouteFragmentRegistry>>,
) -> Result<HttpResponse, Error> {
    let mut vec = Vec::new();
    for stop in STOP_IDS.iter() {
        let resp = state
            .send(route_fragment_registry::GetRouteFragment::new(
                stop.to_string(),
            ))
            .and_then(|x| x.send(route_fragment_registry::route_fragment::FragmentStatusRequest))
            .await
            .unwrap();
        vec.push(resp);
    }

    Ok(HttpResponse::Ok().json(vec))
}

async fn file(_: HttpRequest) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("vis.html")?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    for x in 0..STOP_IDS.len() - 1 {
        let actor_addr = StopState::create(|_ctx| StopState {
            stop_id: STOP_IDS[x],
            name: None,
            last_check: std::time::Instant::now(),
            last_reparture_diff: None,
            prev_stop: if x == 0 {
                None
            } else {
                Some(String::from(STOP_IDS[x - 1]))
            },
        });
        actor_addr.do_send(UpdateRequest {});
    }

    let rfr = route_fragment_registry::RouteFragmentRegistry::from_registry();

    HttpServer::new(move || {
        App::new()
            .data(rfr.clone())
            .route("/", web::get().to(file))
            .service(web::resource("/stats.json").to(handle_frag_stat))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
