import json
import sys

def file_to_json(name):
    loc = sys.argv[1] + "/" + name
    with open(loc, "r") as f:
        return json.load(f)

def get_keys(name):
    index = file_to_json(name)
    il = list(index.keys())
    il.sort()
    return il

def get_unique_hashes(name):
    index = file_to_json(name)
    all_hashes = set()
    for (_, hashes) in index.items():
        for h in hashes:
            all_hashes.add(h)
    return all_hashes

print("pmem images:", len(get_unique_hashes("pmem.index")))
print("nvme images:", len(get_unique_hashes("nvme.index")))
hybrid_states = get_unique_hashes("states.index")
print("hybrid images:", len(hybrid_states))
print("number of semantic states:", len(get_keys("states.index")))

checkpoint_index = file_to_json("checkpoint.index")
checkpoint_ids = list(checkpoint_index.values())
checkpoint_ids.sort()
pmem_index = file_to_json("pmem.index")
nvme_index = file_to_json("nvme.index")
state_index = file_to_json("states.index")

def hash_time(index):
    times = {}
    for (trace_id, hashes) in index.items():
        for h in hashes:
            if h in times:
                times[h].add(trace_id)
            else:
                times[h] = {trace_id}
    return times

pmem_hash_time = hash_time(pmem_index)
nvme_hash_time = hash_time(nvme_index)

def hybrid_hash_time(hybrid_hash):
    pmem_part, nvme_part = hybrid_hash.split('_')
    valid_times = pmem_hash_time[pmem_part] & nvme_hash_time[nvme_part]
    return valid_times

state_time = {}
for state_hash, hybrid_hashes in state_index.items():
    current = set()
    for h in hybrid_hashes:
        times = hybrid_hash_time(h)
        current |= times
    state_time[state_hash] = current

def id_to_prev_checkpoint_value(trace_id):
    for i, cid in enumerate(checkpoint_ids):
        if int(trace_id) <= int(cid):
            return max(i - 1, 0)

state_time_cp = {}
for state_hash, trace_ids in state_time.items():
    state_time_cp[state_hash] = {id_to_prev_checkpoint_value(tid) for tid in trace_ids}

num_states_for_cp = {}
for val in range(len(checkpoint_ids) - 1):
    num_states_for_cp[val] = sum(1 if val in cids else 0 for cids in state_time_cp.values())

COL_GREEN = '\033[92m'
COL_RED = '\033[91m'
COL_END = '\033[0m'

print()
print("number of semantic states per logical operation:")
for val in range(len(checkpoint_ids) - 1):
    print(f"[{val}..{val+1}]: {num_states_for_cp[val]}", end="")
    if num_states_for_cp[val] <= 2:
        print(f" -> {COL_GREEN}atomic{COL_END}")
    else:
        print(f" -> {COL_RED}not atomic{COL_END}")

print()
print("single final state:")
for val in range(len(checkpoint_ids)):
    states_at_exactly_this_cp = set()
    for state_hash, trace_ids in state_time.items():
        if checkpoint_ids[val] in trace_ids:
            states_at_exactly_this_cp.add(state_hash)
    sfs = len(states_at_exactly_this_cp) <= 1
    msg = f"{COL_GREEN}SFS{COL_END}" if sfs else f"{COL_RED}not SFS{COL_END}"
    print(f"checkpoint {val}: {msg}")
