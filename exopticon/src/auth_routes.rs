// Disable this lint because we pass by value to implement the
// actix-web interface.
#![allow(clippy::needless_pass_by_value)]

use actix_identity::{Identity, RequestIdentity};
use actix_service::{Service, Transform};
use actix_web::{
    dev::ServiceRequest, dev::ServiceResponse, http, web::Data, web::Json, Error, HttpResponse,
    ResponseError,
};
use futures::future::{ok, Either, Future, FutureResult};
use futures::Poll;

use crate::app::RouteState;
use crate::auth_handler::AuthData;

/// Route to make login attempt
pub fn login(
    auth_data: Json<AuthData>,
    id: Identity,
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    state
        .db
        .send(auth_data.into_inner())
        .from_err()
        .and_then(move |res| match res {
            Ok(slim_user) => {
                id.remember(slim_user.id.to_string());
                Ok(HttpResponse::Ok().into())
            }
            Err(err) => Ok(err.error_response()),
        })
}

/// Route to make logout attempt
pub fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().into()
}

/// Struct implementing Authentication middleware for api
pub struct WebAuth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for WebAuth
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = WebAuthMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(WebAuthMiddleware { service })
    }
}

/// struct representing authentication middleware for user facing routes
pub struct WebAuthMiddleware<S> {
    /// current service to check authentication for
    service: S,
}

impl<S, B> Service for WebAuthMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, FutureResult<Self::Response, Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        if let Some(user_id) = req.get_identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            Either::A(self.service.call(req))
        } else if req.path() == "/login" {
            Either::A(self.service.call(req))
        } else {
            Either::B(ok(req.into_response(
                HttpResponse::Found()
                    .header(http::header::LOCATION, "/login")
                    .finish()
                    .into_body(),
            )))
        }
    }
}

/// Struct implementing authentication middleware
pub struct Auth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for Auth
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}

/// Struct implementing authentication middleware for api routes
pub struct AuthMiddleware<S> {
    /// service to check authentication for
    service: S,
}

impl<S, B> Service for AuthMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, FutureResult<Self::Response, Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        if let Some(user_id) = req.get_identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            Either::A(self.service.call(req))
        } else {
            Either::B(ok(
                req.into_response(HttpResponse::Unauthorized().finish().into_body())
            ))
        }
    }
}
