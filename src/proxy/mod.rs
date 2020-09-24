use web3::{transports, Web3};
use async_trait::async_trait;
use hyper::{Request, Body, Response};

mod router;
mod hyper_helpers;
mod input_checker;

#[async_trait]
pub trait RouterTrait {
    async fn route(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error>;
}

pub struct Router {
    pub web3: Web3<transports::Http>,
}

#[async_trait]
impl RouterTrait for Router {
    async fn route(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        router::route_request(self.web3.clone(), req).await
    }
}
