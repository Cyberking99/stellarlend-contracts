use soroban_sdk::{Env, Symbol};

pub struct ReentrancyGuard<'a> {
    env: &'a Env,
}

impl<'a> ReentrancyGuard<'a> {
    pub fn new(env: &'a Env) -> Result<Self, u32> {
        let key = Symbol::new(env, "REENTRANCY_LOCK");
        if env.storage().temporary().has(&key) {
            // Error code 7 corresponds to Reentrancy in all operation error enums
            return Err(7);
        }
        env.storage().temporary().set(&key, &true);
        Ok(Self { env })
    }
}

impl<'a> Drop for ReentrancyGuard<'a> {
    fn drop(&mut self) {
        let key = Symbol::new(self.env, "REENTRANCY_LOCK");
        self.env.storage().temporary().remove(&key);
    }
}
