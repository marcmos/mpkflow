use actix::prelude::*;

#[path = "route_fragment.rs"]
pub mod route_fragment;

use std::collections::HashMap;

#[derive(Default)]
pub struct RouteFragmentRegistry {
    route_fragments: HashMap<String, Addr<route_fragment::RouteFragment>>,
}

impl Actor for RouteFragmentRegistry {
    type Context = Context<RouteFragmentRegistry>;
}

impl Supervised for RouteFragmentRegistry {}
impl ArbiterService for RouteFragmentRegistry {}

#[derive(Message)]
#[rtype(result = "Addr<route_fragment::RouteFragment>")]
pub struct GetRouteFragment {
    id: String,
}

impl GetRouteFragment {
    pub fn new(id: String) -> GetRouteFragment {
        GetRouteFragment { id: id }
    }
}

impl Handler<GetRouteFragment> for RouteFragmentRegistry {
    type Result = Addr<route_fragment::RouteFragment>;

    fn handle(
        &mut self,
        _msg: GetRouteFragment,
        _ctx: &mut Context<Self>,
    ) -> Addr<route_fragment::RouteFragment> {
        match self.route_fragments.get(&_msg.id) {
            Some(trip) => trip.clone(),
            None => {
                let id = String::from(&_msg.id);
                let new_fragment = route_fragment::RouteFragment::create(|_| {
                    route_fragment::RouteFragment::new(String::from(_msg.id), String::from("?"))
                });

                self.route_fragments.insert(id, new_fragment.clone());

                new_fragment
            }
        }
    }
}
