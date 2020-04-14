use actix::prelude::*;

use chrono::NaiveTime;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

pub struct RouteFragment {
    stop_names: [String; 2],
    past_trip_duration: Vec<Duration>,
    current_trip_starts: HashMap<String, NaiveTime>,
    current_trip_stops: HashMap<String, NaiveTime>,
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
            current_trip_stops: HashMap::new(),
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct FragmentEntryEvent {
    pub trip_id: String,
    pub instant: Instant,
    pub time: NaiveTime,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct FragmentLeaveEvent {
    pub trip_id: String,
    pub instant: Instant,
    pub time: NaiveTime,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum UpdateMeta {
    UpdateStartName(String),
    UpdateStopName(String),
}

impl RouteFragment {
    fn insert_finished_trip(&mut self, start_time: &NaiveTime, stop_time: &NaiveTime) {
        self.past_trip_duration
            .push((*stop_time - *start_time).to_std().unwrap());
    }
}

impl Handler<FragmentEntryEvent> for RouteFragment {
    type Result = ();

    fn handle(&mut self, msg: FragmentEntryEvent, _ctx: &mut Context<Self>) {
        if let Some(stop) = self.current_trip_stops.get(&msg.trip_id).cloned() {
            self.insert_finished_trip(&msg.time, &stop);
            self.current_trip_stops.remove(&msg.trip_id);
            println!(
                "Unregistered trip (rev-order) {} for fragment {}-{}, took: {:?}",
                msg.trip_id,
                self.stop_names[0],
                self.stop_names[1],
                self.past_trip_duration.last().map(|x| x.as_secs())
            )
        } else {
            println!(
                "Registered new trip {} for fragment {}",
                &msg.trip_id, &self.stop_names[0]
            );
            self.current_trip_starts.insert(msg.trip_id, msg.time);
        }
    }
}

impl Handler<FragmentLeaveEvent> for RouteFragment {
    type Result = ();

    fn handle(&mut self, msg: FragmentLeaveEvent, _ctx: &mut Context<Self>) {
        if let Some(start) = self.current_trip_starts.get(&msg.trip_id).cloned() {
            self.insert_finished_trip(&start, &msg.time);
            self.current_trip_starts.remove(&msg.trip_id);
            println!(
                "Unregistered trip {} for fragment {}-{}, took: {:?}",
                msg.trip_id,
                self.stop_names[0],
                self.stop_names[1],
                self.past_trip_duration.last().map(|x| x.as_secs())
            )
        } else {
            self.current_trip_stops.insert(msg.trip_id, msg.time);
        }
    }
}

impl Handler<UpdateMeta> for RouteFragment {
    type Result = ();

    fn handle(&mut self, msg: UpdateMeta, _ctx: &mut Context<Self>) {
        match msg {
            UpdateMeta::UpdateStartName(start) => self.stop_names[0] = start,
            UpdateMeta::UpdateStopName(stop) => self.stop_names[1] = stop,
        }
    }
}

impl RouteFragment {
    pub fn update_start(s: &str) -> UpdateMeta {
        UpdateMeta::UpdateStartName(s.to_string())
    }

    pub fn update_stop(s: &str) -> UpdateMeta {
        UpdateMeta::UpdateStopName(s.to_string())
    }
}
