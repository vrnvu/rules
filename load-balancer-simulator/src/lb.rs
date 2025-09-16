//! Load Balancer core types and trait

/// Server
#[derive(Debug, Clone)]
pub struct Server {
    pub id: usize,
    pub state: ServerState,
}

/// Server health states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Healthy,
    Unhealthy,
}

/// Load balancer result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadBalancerResult {
    Selected { id: usize },
    NoHealthyServers,
}

/// Load Balancer trait
pub trait LoadBalancer {
    fn select_server(&mut self) -> LoadBalancerResult;
    fn healthy_server(&mut self, server_id: usize);
    fn unhealthy_server(&mut self, server_id: usize);
    fn count(&self) -> usize;
}
