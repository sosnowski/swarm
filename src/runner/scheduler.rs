use tokio::{time::Instant};

use crate::config::{Workload};

pub struct Scheduler {
    workload: Workload,
    started_at: Instant,
    done: bool,
}

impl Scheduler {
    pub fn new(workload: Workload) -> Scheduler {
        return Scheduler {
            workload,
            started_at: Instant::now(),
            done: false,
        }
    }
}

impl Iterator for Scheduler {

    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let elapsed: usize = self.started_at.elapsed().as_secs().try_into().unwrap();

        match self.workload {
            Workload::Constant { duration, max_users } => {
                if elapsed > duration {
                    self.done = true;
                    return None;
                } else {
                    return Some(max_users);
                }
            },
            Workload::Linear { duration, max_users, ramp_up_time } => {
                if elapsed > duration {
                    self.done = true;
                    return None;
                } else {
                    if elapsed >= ramp_up_time {
                        return Some(max_users);
                    }
                    let mut res = (elapsed * max_users) / ramp_up_time;
                    if res == 0 {
                        res = 1;
                    }
                    return Some(res);
                }
            },
            Workload::EaseOut { duration, max_users, ramp_up_time } => todo!(),
            Workload::Sin { duration, max_users, min_users, cycle_time } => todo!(),
        };
    }
}