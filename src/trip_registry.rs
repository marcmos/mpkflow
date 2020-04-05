use actix::prelude::*;

#[path = "passage.rs"]
mod passage;

use std::collections::HashMap;

#[derive(Default)]
pub struct TripRegistry {
    trips: HashMap<String, Addr<passage::Trip>>,
}

impl Actor for TripRegistry {
    type Context = Context<TripRegistry>;
}

impl Supervised for TripRegistry {}
impl ArbiterService for TripRegistry {}

#[derive(Message)]
#[rtype(result = "Addr<passage::Trip>")]
pub struct RegisterTrip {
    id: String,
}

impl RegisterTrip {
    pub fn new(id: String) -> RegisterTrip {
        RegisterTrip { id: id }
    }
}

impl Handler<RegisterTrip> for TripRegistry {
    type Result = Addr<passage::Trip>;

    fn handle(&mut self, _msg: RegisterTrip, _ctx: &mut Context<Self>) -> Addr<passage::Trip> {
        match self.trips.get(&_msg.id) {
            Some(trip) => trip.clone(),
            None => {
                let new_trip =
                    passage::Trip::create(|_| passage::Trip::new(String::from(&_msg.id)));

                self.trips.insert(_msg.id, new_trip.clone());

                new_trip
            }
        }
    }
}
