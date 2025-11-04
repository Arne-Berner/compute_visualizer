/// Used in the merge step


@group(0) @binding(0)
var<storage, read_write> labels: array<atomic<u32>>;

/// Find goes through the parent nodes recursively, until it finds the root node.
/// This works, because root points to itself. Usually this would be done in place,
/// but wgsl does not allow mutable function parameters.
fn Find(n: u32) -> u32 {
    var idx = n;

    loop {
        let parent = atomicLoad(&labels[idx]);
        if (parent == idx) {
            break;
        }
        idx = parent;

    }
    return idx;
}

fn FindAndCompress(n: u32) -> u32 {
    var idx = n;
    loop {
        let parent = atomicLoad(&labels[idx]);
        if (parent == idx) {
            break;
        }
        idx = parent;
        atomicStore(&labels[n], idx);
    }
    return idx;
}

fn Union(a_in: u32, b_in: u32) {
    var a: u32 = a_in;
    var b: u32 = b_in;
    var done: bool = false;

    loop {
        // Find current roots
        a = Find(a);
        b = Find(b);

        if (a < b) {
            // atomicMin takes i32 and returns the previous i32 value
            let old = atomicMin(&labels[b], a);
            if old == b {
                break;
            }
            b = old;
        } else if (b < a) {
            let old = atomicMin(&labels[a], b);
            if old == a {
                break;
            }
            a = old;
        } else {
            break;
        }
    }
}
