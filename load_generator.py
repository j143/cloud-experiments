#!/usr/bin/env python3
"""
Production-grade Load Generator for CAP Theorem Research
Generates configurable workloads for Cosmos DB consistency testing
"""

import asyncio, time, random, json, uuid
from typing import Dict, List, Callable, Optional
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum

class LoadProfile(Enum):
    CONSTANT = "constant"
    RAMP_UP = "ramp_up"
    BURST = "burst"
    REALISTIC = "realistic"

class OperationType(Enum):
    CREATE = "create"
    READ = "read"
    UPDATE = "update"
    DELETE = "delete"

@dataclass
class Operation:
    op_type: OperationType
    doc_id: str
    partition_key: str
    payload: Dict
    timestamp: float = field(default_factory=time.time)
    consistency_level: str = "Session"
    doc_type: str = "standard"  # critical/important/non-critical

@dataclass
class LoadConfig:
    target_ops_per_sec: int = 100
    duration_seconds: int = 60
    profile: LoadProfile = LoadProfile.CONSTANT
    read_write_ratio: float = 0.8  # 80% reads, 20% writes
    num_threads: int = 10
    doc_count: int = 1000
    payload_size_bytes: int = 1024
    
class WorkloadGenerator:
    """Multi-threaded workload generator for distributed systems testing"""
    
    def __init__(self, config: LoadConfig):
        self.config = config
        self.operation_queue: asyncio.Queue = asyncio.Queue()
        self.metrics = {
            'total_ops': 0,
            'ops_per_sec': [],
            'operation_types': {op.value: 0 for op in OperationType}
        }
        self.start_time = None
        self.doc_ids = [f"doc-{i}" for i in range(config.doc_count)]
    
    def generate_payload(self) -> Dict:
        """Generate realistic test payload"""
        return {
            'id': str(uuid.uuid4()),
            'data': 'x' * self.config.payload_size_bytes,
            'timestamp': datetime.utcnow().isoformat(),
            'sequence': random.randint(1, 1000000)
        }
    
    def get_operation_type(self) -> OperationType:
        """Determine operation based on read/write ratio"""
        if random.random() < self.config.read_write_ratio:
            return OperationType.READ
        else:
            return [OperationType.CREATE, OperationType.UPDATE][random.randint(0, 1)]
    
    def get_load_rate(self, elapsed_seconds: float) -> int:
        """Calculate target ops/sec based on load profile"""
        profile = self.config.profile
        target = self.config.target_ops_per_sec
        
        if profile == LoadProfile.CONSTANT:
            return target
        elif profile == LoadProfile.RAMP_UP:
            progress = min(elapsed_seconds / 300, 1.0)  # Ramp over 300s
            return int(target * progress)
        elif profile == LoadProfile.BURST:
            if elapsed_seconds < 100:
                return target
            elif elapsed_seconds < 200:
                return target * 50  # 50x burst
            else:
                return target
        elif profile == LoadProfile.REALISTIC:
            # Simulate realistic diurnal pattern
            hour = (elapsed_seconds % 3600) / 3600
            return int(target * (0.5 + 0.5 * (1 + 0.5 * __import__('math').sin(2 * 3.14159 * hour))))
        return target
    
    def create_operation(self) -> Operation:
        """Create a single operation"""
        op_type = self.get_operation_type()
        doc_id = random.choice(self.doc_ids)
        
        # Classify document type for hybrid consistency
        doc_type_roll = random.random()
        if doc_type_roll < 0.2:
            doc_type = "critical"
            consistency = "Strong"
        elif doc_type_roll < 0.5:
            doc_type = "important"
            consistency = "Session"
        else:
            doc_type = "non-critical"
            consistency = "Eventual"
        
        return Operation(
            op_type=op_type,
            doc_id=doc_id,
            partition_key=doc_type,
            payload=self.generate_payload(),
            consistency_level=consistency,
            doc_type=doc_type
        )
    
    async def run_workload(self, callback: Callable[[Operation], None]):
        """Execute workload with configurable load profile"""
        self.start_time = time.time()
        tasks = []
        
        for _ in range(self.config.num_threads):
            task = asyncio.create_task(self._worker_loop(callback))
            tasks.append(task)
        
        await asyncio.gather(*tasks)
    
    async def _worker_loop(self, callback: Callable[[Operation], None]):
        """Worker thread that generates operations at target rate"""
        last_print = time.time()
        window_ops = 0
        
        while (time.time() - self.start_time) < self.config.duration_seconds:
            elapsed = time.time() - self.start_time
            target_rate = self.get_load_rate(elapsed)
            
            # Rate limiting: sleep if needed to match target
            target_interval = 1.0 / target_rate if target_rate > 0 else 0.01
            
            op = self.create_operation()
            await callback(op)
            
            self.metrics['total_ops'] += 1
            self.metrics['operation_types'][op.op_type.value] += 1
            window_ops += 1
            
            # Print throughput every 10 seconds
            now = time.time()
            if (now - last_print) >= 10:
                actual_ops_per_sec = window_ops / (now - last_print)
                self.metrics['ops_per_sec'].append(actual_ops_per_sec)
                print(f"[{elapsed:.1f}s] Ops/sec: {actual_ops_per_sec:.1f} (target: {target_rate})")
                window_ops = 0
                last_print = now
            
            await asyncio.sleep(target_interval)
    
    def get_stats(self) -> Dict:
        """Return comprehensive workload statistics"""
        avg_ops = sum(self.metrics['ops_per_sec']) / len(self.metrics['ops_per_sec']) if self.metrics['ops_per_sec'] else 0
        return {
            'total_operations': self.metrics['total_ops'],
            'average_ops_per_sec': avg_ops,
            'operation_breakdown': self.metrics['operation_types'],
            'duration_seconds': self.config.duration_seconds
        }

# Usage example
if __name__ == "__main__":
    config = LoadConfig(
        target_ops_per_sec=100,
        duration_seconds=60,
        profile=LoadProfile.CONSTANT,
        num_threads=5
    )
    
    generator = WorkloadGenerator(config)
    
    async def test_callback(op: Operation):
        pass  # Will be implemented by measurement engine
    
    print(f"Starting load generation: {config.target_ops_per_sec} ops/sec")
    asyncio.run(generator.run_workload(test_callback))
    print(json.dumps(generator.get_stats(), indent=2))
