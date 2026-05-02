import pathlib 
from pathlib import Path 
import re
import sys

# TODO: Needed data:
# total tasks completed
# makespan (?)
# average wait time
# average turnaround time 

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Provide a (relative) file to open.")
        sys.exit(1)

    filepath = Path.cwd() / sys.argv[1]
    
    if not filepath.exists():
        print("`out.txt` not found. Try running the program first.")
        sys.exit(1)

    s: str = ''
    with open(filepath) as f:
        s = f.read()

    matches = re.findall(r"Finished.+?([A-z]+)\stask\s([0-9]+).+?([0-9.]+).+?About\s([0-9.]+)", s)
    load_avg_match = re.search(r"load:\s([0-9]+)", s)

    for i in range(len(matches)):
        t = matches[i]
        matches[i] = (t[0], int(t[1]), float(t[3]) - float(t[2]), float(t[3]))

    print("ID\tKind\tWT (s)\tTAT (s)")
    for task_kind, task_id, wait_time_s, turnaround_time_s in matches:
        print(f"{task_id}\t{task_kind}\t{wait_time_s:.3}s\t{turnaround_time_s:.3}s")
    print("ID\tKind\tWT (s)\tTAT (s)")

    wait_time_s_avg = sum([t[2] for t in matches]) / len(matches)
    turnaround_time_s_avg = sum([t[3] for t in matches]) / len(matches)
    burst_time_s_avg = sum([t[3] - t[2] for t in matches]) / len(matches)
    makespan = re.search(r"Finished\sall\sjobs\sin\s([0-9.]+)", s).group(1);

    print(f"Total tasks:        \t{len(matches)}")
    print(f"      (CPU):        \t{len([t for t in matches if t[0] == 'CPU'])}")
    print(f"       (IO):        \t{len([t for t in matches if t[0] == 'IO'])}")
    print(f"Average wait time:  \t{wait_time_s_avg:.3}s")
    print(f"Average turnaround: \t{turnaround_time_s_avg:.3}s")
    print(f"Average burst time: \t{burst_time_s_avg:.3}s")
    print(f"Makespan:           \t{makespan}s")
    print(f"Avg. load:          \t{load_avg_match.group(1)}")
