use crate::synth::{Gate2, Op};
use std::collections::{HashMap, HashSet, VecDeque};

/// Line architecture: 0-1-2-3
pub fn line_hardware() -> HashMap<usize, Vec<usize>> {
    let mut adj = HashMap::new();
    adj.insert(0, vec![1]);
    adj.insert(1, vec![0, 2]);
    adj.insert(2, vec![1, 3]);
    adj.insert(3, vec![2]);
    adj
}

/// T-shape architecture:
///      1
///      |
/// 0 -- 2 -- 3
pub fn t_shape_hardware() -> HashMap<usize, Vec<usize>> {
    let mut adj = HashMap::new();
    adj.insert(0, vec![2]);
    adj.insert(1, vec![2]);
    adj.insert(2, vec![0, 1, 3]);
    adj.insert(3, vec![2]);
    adj
}

pub fn find_path(
    start: usize,
    end: usize,
    hardware: &HashMap<usize, Vec<usize>>,
) -> Option<Vec<usize>> {
    let mut queue = VecDeque::new();
    let mut parent: HashMap<usize, usize> = HashMap::new();
    let mut visited: HashSet<usize> = HashSet::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        if current == end {
            let mut path = vec![end];
            let mut c = end;
            while let Some(&p) = parent.get(&c) {
                path.push(p);
                c = p;
            }
            path.reverse();
            return Some(path);
        }

        for &neighbor in &hardware[&current] {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                parent.insert(neighbor, current);
                queue.push_back(neighbor);
            }
        }
    }

    None
}

fn swap_sequence(a: usize, b: usize) -> Vec<Op> {
    vec![
        Op::Gate2(Gate2::CX, a, b),
        Op::Gate2(Gate2::CX, b, a),
        Op::Gate2(Gate2::CX, a, b),
    ]
}

pub fn route_cnot(control: usize, target: usize, hardware: &HashMap<usize, Vec<usize>>) -> Vec<Op> {
    let mut ops = Vec::new();

    if hardware[&control].contains(&target) {
        ops.push(Op::Gate2(Gate2::CX, control, target));
        return ops;
    }

    let path = find_path(control, target, hardware).unwrap();

    for i in 0..path.len() - 2 {
        let a = path[i];
        let b = path[i + 1];
        ops.extend(swap_sequence(a, b));
    }

    let n = path.len();
    ops.push(Op::Gate2(Gate2::CX, path[n - 2], target));

    for i in (0..path.len() - 2).rev() {
        let a = path[i];
        let b = path[i + 1];
        ops.extend(swap_sequence(a, b));
    }

    ops
}

pub fn route_circuit(ops: Vec<Op>, hardware: &HashMap<usize, Vec<usize>>) -> Vec<Op> {
    let mut routed_ops = Vec::new();

    for op in ops {
        match op {
            Op::Gate2(Gate2::CX, c, t) => {
                let routed = route_cnot(c, t, hardware);
                routed_ops.extend(routed);
            }
            _ => routed_ops.push(op),
        }
    }

    routed_ops
}
