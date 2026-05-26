import multiprocessing
import time
import math
import os

def cpu_wave_task(core_index):
    """
    让单个 CPU 核心的占用率按 20%, 40%, 60%, 80%, 100% 阶梯式递增，每阶段持续 3 秒，然后循环
    """
    print(f"进程 {multiprocessing.current_process().name} (PID: {os.getpid()}) 已启动。")
    
    # 周期控制参数
    STAGE_DURATION = 10.0  # 每个阶段持续 3 秒
    NUM_STAGES = 5        # 总共有 5 个阶段 (20%, 40%, 60%, 80%, 100%)
    TOTAL_PERIOD = STAGE_DURATION * NUM_STAGES  # 一个完整循环周期为 15 秒
    INTERVAL = 0.05       # 微观控制周期为 50 毫秒
    
    start_time = time.time()
    
    try:
        while True:
            current_time = time.time() - start_time
            
            # 1. 计算当前时间在 15 秒周期内的位置
            cycle_time = current_time % TOTAL_PERIOD
            
            # 2. 计算当前属于第几个阶段 (0 到 4)
            # 使用 min 确保由于浮点数精度导致的边界问题不会溢出
            stage = min(int(cycle_time // STAGE_DURATION), NUM_STAGES - 1)
            
            # 3. 根据阶段计算目标 CPU 占用率 (0.2, 0.4, 0.6, 0.8, 1.0)
            target_cpu_ratio = 0.2 + 0.2 * stage
            
            # 4. 在这 50 毫秒的微观周期内，计算应该工作多久，休息多久
            run_time = INTERVAL * target_cpu_ratio
            sleep_time = INTERVAL - run_time
            
            # 5. 开始执行密集的数学计算（工作阶段）
            work_end_time = time.time() + run_time
            while time.time() < work_end_time:
                _ = 1000 * 1000  # 纯粹消耗 CPU 的计算
                
            # 6. 释放 CPU（休息阶段）
            if sleep_time > 0:
                time.sleep(sleep_time)
                
    except KeyboardInterrupt:
        # 捕获子进程中的退出信号
        pass

if __name__ == "__main__":
    cpu_count = multiprocessing.cpu_count()
    print(f"检测到系统有 {cpu_count} 个 CPU 核心。")
    print("程序已启动！打开任务管理器或活动监视器，观察 CPU 性能曲线（会呈现阶梯状规律性波动）。")
    print("随时按 Ctrl + C 可以安全停止。")
    print("-" * 50)
    
    time.sleep(2)
    processes = []
    
    try:
        # 为每个核心创建一个波动进程
        for i in range(cpu_count):
            p = multiprocessing.Process(target=cpu_wave_task, args=(i,), name=f"WaveCore-{i}")
            processes.append(p)
            p.start()

        # 让主线程等待
        for p in processes:
            p.join()

    except KeyboardInterrupt:
        print("\n正在接收停止信号，正在释放 CPU...")
        for p in processes:
            p.terminate()
            p.join()
        print("所有测试进程已关闭，系统恢复正常。")