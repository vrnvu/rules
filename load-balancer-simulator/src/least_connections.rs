use crate::{LoadBalancer, LoadBalancerResult, Server, ServerState};

pub struct LeastConnections {
    servers: Vec<Server>,
    connection_counts: Vec<usize>,
}

impl LeastConnections {
    pub fn new(servers: Vec<Server>) -> Self {
        assert!(!servers.is_empty());
        let connection_counts = vec![0; servers.len()];

        Self {
            servers,
            connection_counts,
        }
    }
}

impl LoadBalancer for LeastConnections {
    fn select_server(&mut self) -> LoadBalancerResult {
        assert!(!self.servers.is_empty());
        assert!(self.connection_counts.len() == self.servers.len());

        let mut best_server = None;
        let mut min_connections = usize::MAX;

        for (i, server) in self.servers.iter().enumerate() {
            if server.state == ServerState::Healthy {
                if self.connection_counts[i] < min_connections {
                    min_connections = self.connection_counts[i];
                    best_server = Some(i);
                }
            }
        }

        match best_server {
            Some(server_id) => {
                self.connection_counts[server_id] += 1;
                LoadBalancerResult::Selected { id: server_id }
            }
            None => LoadBalancerResult::NoHealthyServers,
        }
    }

    fn healthy_server(&mut self, server_id: usize) {
        assert!(server_id < self.servers.len());
        assert!(self.connection_counts.len() == self.servers.len());

        self.servers[server_id].state = ServerState::Healthy;
    }

    fn unhealthy_server(&mut self, server_id: usize) {
        assert!(server_id < self.servers.len());
        assert!(self.connection_counts.len() == self.servers.len());

        self.servers[server_id].state = ServerState::Unhealthy;
        self.connection_counts[server_id] = 0;
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
        let _ = LeastConnections::new(vec![]);
    }

    #[test]
    fn test_new_one_server() {
        let lb = LeastConnections::new(vec![Server {
            id: 0,
            state: ServerState::Healthy,
        }]);
        assert_eq!(lb.count(), 1);
    }

    #[test]
    fn test_select_server() {
        let mut lb = LeastConnections::new(vec![Server {
            id: 0,
            state: ServerState::Healthy,
        }]);
        assert_eq!(lb.count(), 1);

        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });

        lb.unhealthy_server(0);
        assert_eq!(lb.select_server(), LoadBalancerResult::NoHealthyServers);

        lb.healthy_server(0);
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });
    }

    #[test]
    fn test_least_connections_behavior() {
        let mut lb = LeastConnections::new(vec![
            Server {
                id: 0,
                state: ServerState::Healthy,
            },
            Server {
                id: 1,
                state: ServerState::Healthy,
            },
        ]);
        assert_eq!(lb.count(), 2);

        // First selection should pick server 0 (both have 0 connections)
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });

        // Second selection should pick server 1 (server 0 now has 1 connection)
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 1 });

        // Third selection should pick server 0 again (both have 1 connection, pick first)
        assert_eq!(lb.select_server(), LoadBalancerResult::Selected { id: 0 });
    }

    #[test]
    fn test_least_connections_random_sequence() {
        let server_count = 5;
        let seed = 42;
        let count: usize = 100_000;

        let servers = (0..server_count)
            .map(|id| Server {
                id,
                state: ServerState::Healthy,
            })
            .collect();

        let mut lb = LeastConnections::new(servers);
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
