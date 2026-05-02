import os 

if __name__ == '__main__':
    os.system("cargo run --features fifo_pool 2>&1 | tee -a out_fifo.txt")
    os.system("cargo run --features pqueue_pool 2>&1 | tee -a out_pqueue.txt")
    os.system("python3 analyze.py out_fifo.txt > results_fifo.txt")
    os.system("python3 analyze.py out_pqueue.txt > results_pqueue.txt")
