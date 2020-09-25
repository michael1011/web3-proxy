use crate::proxy::{Router, RouterTrait};
use hyper::service::Service;
use hyper::{Body, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Svc {
    router: Router,
}

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let router_clone = self.router.clone();

        Box::pin(async move { router_clone.route(req).await })
    }
}

pub struct MakeSvc {
    pub router: Router,
}

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let router = self.router.clone();
        let future = async move { Ok(Svc { router }) };
        Box::pin(future)
    }
}
