use std::sync::{Arc, Mutex, MutexGuard};

use crate::{
    integrations::Paths,
    web::http::{internal_error, HttpResult},
};

pub(crate) type WebState = Arc<Mutex<Paths>>;

pub(crate) fn new_web_state(paths: Paths) -> WebState {
    Arc::new(Mutex::new(paths))
}

pub(crate) fn lock_paths(state: &WebState) -> HttpResult<MutexGuard<'_, Paths>> {
    state
        .lock()
        .map_err(|err| internal_error(format!("web state lock poisoned: {err}")))
}
