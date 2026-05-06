import subprocess
import threading
import sys
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn
from rich.console import Console

console = Console()

def run_task(command, progress, task_id, stop_event, error_list, output_file=None):
    """
    Executes a command. Writes stdout/stderr to output_file if provided.
    If it fails, sets the stop_event to cancel others.
    """
    if stop_event.is_set():
        progress.update(task_id, description="[grey]Cancelled", visible=False)
        return

    # Use 'None' for stdout/stderr if no file is provided (default behavior)
    out_handle = subprocess.DEVNULL
    err_handle = subprocess.PIPE

    try:
        if output_file:
            out_handle = open(output_file, "w")
            err_handle = out_handle # This mimics '2>&1'

        process = subprocess.Popen(
            command, 
            shell=True, 
            stdout=out_handle, 
            stderr=err_handle,
            text=True
        )

        while process.poll() is None:
            if stop_event.is_set():
                process.terminate()
                progress.update(task_id, description="[red]Terminated")
                return
            threading.Event().wait(0.1)

        if process.returncode != 0:
            # If we didn't redirect to a file, we can read the pipe
            error_msg = f"Command failed: {command}"
            if not output_file:
                error_msg += f"\nError: {process.stderr.read()}"
            else:
                error_msg += f"\nCheck {output_file} for logs."
            
            error_list.append(error_msg)
            stop_event.set()
            progress.update(task_id, description="[bold red]Failed")
        else:
            progress.update(task_id, completed=100, description="[green]Success")

    finally:
        # Ensure the file handle is closed even if the process crashes
        if output_file and out_handle != subprocess.DEVNULL:
            out_handle.close()

def main():
    stop_event = threading.Event()
    error_list = []

    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
        console=console
    ) as progress:

        # --- PHASE 1: Parallel Cargo Runs ---
        # Note: Commands no longer contain shell redirection logic
        fifo_id = progress.add_task("[cyan]Cargo: FIFO Pool", total=100)
        pqueue_id = progress.add_task("[magenta]Cargo: PQueue Pool", total=100)

        t1 = threading.Thread(target=run_task, args=(
            "cargo run --no-default-features --features fifo_pool", 
            progress, fifo_id, stop_event, error_list, "out_fifo.txt"
        ))
        t2 = threading.Thread(target=run_task, args=(
            "cargo run --no-default-features --features dynamic_pool", 
            progress, pqueue_id, stop_event, error_list, "out_dynamic.txt"
        ))

        t1.start()
        t2.start()
        t1.join()
        t2.join()

        if stop_event.is_set():
            progress.stop()
            console.print("\n[bold red]Aborting: Cargo phase failed.[/bold red]")
            for err in error_list:
                console.print(f"[red]{err}[/red]")
            sys.exit(1)

        # --- PHASE 2: Sequential Analysis ---
        # These still use standard shell redirection for the final result files
        ana_fifo_id = progress.add_task("[yellow]Analyze: FIFO", total=100)
        run_task("python3 analyze.py out_fifo.txt > results_fifo.txt", progress, ana_fifo_id, stop_event, error_list)

        if not stop_event.is_set():
            ana_pqueue_id = progress.add_task("[green]Analyze: PQueue", total=100)
            run_task("python3 analyze.py out_dynamic.txt > results_dynamic.txt", progress, ana_pqueue_id, stop_event, error_list)

    if not stop_event.is_set():
        console.print("\n[bold green]✔ All tasks completed successfully.[/bold green]")

if __name__ == "__main__":
    main()
