use crate::{LoadBalancer, LoadBalancerResult, Server, ServerState};

pub struct RoundRobin {
    servers: Vec<Server>,
    current_index: usize,
    unhealthy_count: usize,
}

impl RoundRobin {
    pub fn new(servers: Vec<Server>) -> Self {
        assert!(!servers.is_empty());
        let unhealthy_count = servers
            .iter()
            .filter(|s| s.state == ServerState::Unhealthy)
            .count();

        Self {
            servers,
            current_index: 0,
            unhealthy_count,
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn select_server(&mut self) -> LoadBalancerResult {
        assert!(!self.servers.is_empty());
        assert!(self.current_index < self.servers.len());
        assert!(self.unhealthy_count <= self.servers.len());

        if self.unhealthy_count == self.servers.len() {
            return LoadBalancerResult::NoHealthyServers;
        }

        while self.servers[self.current_index].state == ServerState::Unhealthy {
            self.current_index = (self.current_index + 1) % self.servers.len();
        }

        LoadBalancerResult::Selected {
            id: self.current_index,
        }
    }

    fn healthy_server(&mut self, server_id: usize) {
        assert!(server_id < self.servers.len());
        assert!(self.unhealthy_count <= self.servers.len());

        if self.servers[server_id].state == ServerState::Unhealthy {
            self.unhealthy_count -= 1;
            self.servers[server_id].state = ServerState::Healthy;
        }
    }

    fn unhealthy_server(&mut self, server_id: usize) {
        assert!(server_id < self.servers.len());
        assert!(self.unhealthy_count <= self.servers.len());

        if self.servers[server_id].state == ServerState::Healthy {
            self.unhealthy_count += 1;
            self.servers[server_id].state = ServerState::Unhealthy;
        }
    }

    fn count(&self) -> usize {
        self.servers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Step {
        SelectServer,
        MarkHealthy(usize),
        MarkUnhealthy(usize),
    }

    fn generate_random_steps(seed: u64, count: usize, server_count: usize) -> Vec<Step> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut steps = Vec::with_capacity(count);

        for _ in 0..count {
            let choice = rng.gen_range(0..3);
            let step = match choice {
                0 => Step::SelectServer,
                1 => Step::MarkHealthy(rng.gen_range(0..server_count)),
                _ => Step::MarkUnhealthy(rng.gen_range(0..server_count)),
            };
            steps.push(step);
        }

        steps
    }

    #[test]
    #[should_panic]
    fn test_new_empty_servers_panics() {
        let _ = RoundRobin::new(vec![]);
    }

    #[test]
    fn test_new_one_server() {
        let lb = RoundRobin::new(vec![Server {
            id: 0,
            state: ServerState::Healthy,
        }]);
        assert_eq!(lb.count(), 1);
    }

    #[test]
    fn test_select_server() {
        let mut lb = RoundRobin::new(vec![Server {
            id: 0,
            state: ServerState::Healthy,
        }]);
        assert_eq!(lb.count(), 1);

        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });

        lb.unhealthy_server(0);
        assert_eq!(lb.select_server(), LoadBalancerResult::NoHealthyServers);

        lb.healthy_server(0);
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });
    }

    #[test]
    fn test_round_robin_random_sequence() {
        let server_count = 5;
        let seed = 42;
        let count: usize = 100_000;

        let servers = (0..server_count)
            .map(|id| Server {
                id,
                state: ServerState::Healthy,
            })
            .collect();

        let mut lb = RoundRobin::new(servers);
        let steps = generate_random_steps(seed, count, server_count);

        for step in steps {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match step {
                Step::SelectServer => {
                    lb.select_server();
                }
                Step::MarkHealthy(server_id) => {
                    lb.healthy_server(server_id);
                }
                Step::MarkUnhealthy(server_id) => {
                    lb.unhealthy_server(server_id);
                }
            }));

            assert!(result.is_ok(), "Panic occurred with step: {:?}", step);
        }
    }
}
