/// No authentication has occurred yet.
pub struct Unauthenticated;

/// Authentication has passed. Use this context downstream.
pub struct Authenticated {
    pub token: String,
    // Add claims/user_id here as needed
}

/// Type-state context encoding security progression.
#[derive(Clone)]
pub struct SecurityContext<S> {
    pub state: S,
}

impl SecurityContext<Unauthenticated> {
    pub fn new() -> Self {
        Self {
            state: Unauthenticated,
        }
    }

    /// Transition to Authenticated.
    pub fn authenticate(self, token: String) -> SecurityContext<Authenticated> {
        SecurityContext {
            state: Authenticated { token },
        }
    }
}
