extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use std::time::Duration;

use actix::prelude::*;

mod passage;
mod route_fragment;
mod route_fragment_registry;
mod trip_registry;

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
        println!("{}", self.stop_id);
        let x = actix::fut::wrap_future::<_, Self>(fetch_stop_passage(&self.stop_id));

        ctx.wait(x.map(|_result, actor, _ctx| {
            // println!("{:#?}", result);
            actor.last_reparture_diff = _result
                .as_ref()
                .unwrap()
                .old
                .first()
                .map(|x| x.actual_relative_time);
            let wait = _result
                .as_ref()
                .unwrap()
                .actual
                .first()
                .map(|x| x.actual_relative_time);
            actor.name = Some(String::from(&_result.as_ref().unwrap().stop_name));

            let z = match wait {
                Some(x) if x > 10 => x,
                Some(_) => 10,
                _ => 60,
            };

            println!(
                "{:?} update in {}",
                actor.name.as_ref().unwrap_or(&actor.stop_id.to_string()),
                z as u64
            );
            _ctx.notify_later(UpdateRequest {}, Duration::from_secs(z as u64));

            &_result.unwrap().old.iter().for_each(|x| {
                trip_registry::TripRegistry::from_registry()
                    .do_send(trip_registry::RegisterTrip::new(String::from(&x.trip_id)));
            });
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

const MOGILSKIE_POKOJU: &str = "12529";
const GRZEGORZECKIE: &str = "36519";
const CZYZYNSKIE: &str = "40849";
const DOKTOR_LUBICZ: &str = "12629";
const STOP_IDS: [&str; 3] = [MOGILSKIE_POKOJU, GRZEGORZECKIE, CZYZYNSKIE];
// const STOP_IDS: [&str; 3] = ["12919", "13019", "304019"];

// struct TripTracker {
// trips: HashMap<String, Addr<Trip>>,
// }

fn main() {
    System::run(|| {
        // trip_registry::TripRegistry::start_default()
        // .do_send(trip_registry::RegisterTrip { id: String::from("8059232507168034829") });

        STOP_IDS.iter().for_each(|x| {
            StopState::create(|_ctx| StopState {
                stop_id: x,
                name: None,
                last_check: std::time::Instant::now(),
                last_reparture_diff: None,
            })
            .do_send(UpdateRequest {});
        });

        // passage::Trip::create(|_ctx| {
        //     passage::Trip {
        //         id: String::from("8059232507169583113"),
        //         next_stop: None,
        //         stop_seq: None,
        //         trip_meta: None,
        //         update_time: None,
        //     }
        // });

        // let addr = StopState::create(|_ctx| {
        //     StopState {
        //         stop_id: "281129",
        //         last_check: std::time::Instant::now(),
        //         last_reparture_diff: 1
        //     }
        // });
        // addr.do_send(PrintState {});
        // addr.do_send(UpdateRequest {});
        // addr.do_send(PrintState {});
    })
    .unwrap();
}

/*
1. zrobić registry dla kolejnych fragmentów linii tramwajowych
2. zapisywać w stanie tego aktora czasy zakończonych przejazdów (dla przeszłych) i momenty wjechania – aktualnych
3. do aktora wysyłać tylko komunikaty wjechania i wyjechania
4. do renderowania stanu aktualnego używać tylko tych aktorów
*/
