import pathlib 
from pathlib import Path 
import re
import sys
import matplotlib.pyplot as plt 
import numpy as np 

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
        print(f"`{filepath}` not found. Try running the program first.")
        sys.exit(1)

    s: str = ''
    with open(filepath) as f:
        s = f.read()

    matches = re.findall(r"Finished.+?([A-z]+)\stask\s([0-9]+).+?([0-9.]+).+?About\s([0-9.]+)", s)
    load_avg_match = re.search(r"load:\s([0-9]+)", s)

    for i in range(len(matches)):
        t = matches[i]
        matches[i] = (t[0], int(t[1]), float(t[3]) - float(t[2]), float(t[3]))

    #print("ID\tKind\tWT (s)\tTAT (s)")
    #for task_kind, task_id, wait_time_s, turnaround_time_s in matches:
    #    print(f"{task_id}\t{task_kind}\t{abs(wait_time_s):.3f}s\t{abs(turnaround_time_s):.3f}s")
    #print("ID\tKind\tWT (s)\tTAT (s)")

    
    wait_times_s = [t[2] for t in matches]
    wait_time_max_s = max(wait_times_s)
    wait_time_s_avg = sum(wait_times_s) / len(wait_times_s)

    turnaround_time_s_avg = sum([t[3] for t in matches]) / len(matches)
    burst_time_s_avg = sum([t[3] - t[2] for t in matches]) / len(matches)
    makespan = re.search(r"Finished\sall\sjobs\sin\s([0-9.]+)", s).group(1);

    fig, ax = plt.subplots()

    task_labels = [i+1 for i in range(len(matches))]
    turnaround_per_task_s = [t[3] for t in matches]
    ax.bar(task_labels, turnaround_per_task_s)
    ax.set_ylabel("TAT (s)")
    ax.set_xlabel("Task (ID) in order of arrival time")
    ax.set_title("Turnaround time per task")

    plt_out = str(filepath).replace(filepath.suffix, ".png")
    plt.savefig(plt_out)
    print(f"Plot saved to {plt_out}")

