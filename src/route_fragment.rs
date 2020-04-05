use actix::prelude::*;

use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

pub struct RouteFragment {
    stop_names: [String; 2],
    past_trip_duration: Vec<Duration>,
    current_trip_starts: HashMap<String, Instant>,
}

impl Actor for RouteFragment {
    type Context = Context<RouteFragment>;
}

impl RouteFragment {
    pub fn new(start_name: String, stop_name: String) -> RouteFragment {
        RouteFragment {
            stop_names: [start_name, stop_name],
            past_trip_duration: Vec::new(),
            current_trip_starts: HashMap::new(),
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct FragmentEntryEvent {
    pub trip_id: String,
    pub instant: Instant,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct FragmentLeaveEvent {
    pub trip_id: String,
    pub instant: Instant,
}

impl Handler<FragmentEntryEvent> for RouteFragment {
    type Result = ();

    fn handle(&mut self, msg: FragmentEntryEvent, _ctx: &mut Context<Self>) {
        println!(
            "Registered new trip {} for fragment {}",
            &msg.trip_id, &self.stop_names[0]
        );
        self.current_trip_starts.insert(msg.trip_id, msg.instant);
    }
}

impl Handler<FragmentLeaveEvent> for RouteFragment {
    type Result = ();

    fn handle(&mut self, msg: FragmentLeaveEvent, _ctx: &mut Context<Self>) {
        if let Some(start) = self.current_trip_starts.get(&msg.trip_id) {
            self.past_trip_duration
                .push(msg.instant.duration_since(*start));
            self.current_trip_starts.remove(&msg.trip_id);
            println!(
                "Unregistered trip {} for fragment {}, took: {:?}",
                msg.trip_id,
                self.stop_names[0],
                self.past_trip_duration.last().map(|x| x.as_secs())
            )
        } else {
            println!(
                "Lost leave due to missing {} start event for stop {}",
                &msg.trip_id, &self.stop_names[0]
            );
        }
    }
}
