#!/usr/bin/env python3
"""
CAP Theorem Measurement Engine
Measures consistency, availability, latency, and throughput for Cosmos DB
"""

import time, statistics, json, os
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from azure.cosmos import CosmosClient, PartitionKey
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

@dataclass
class LatencyMetrics:
    p50: float = 0.0
    p95: float = 0.0
    p99: float = 0.0
    min: float = 0.0
    max: float = 0.0
    avg: float = 0.0
    stddev: float = 0.0
    all_values: List[float] = field(default_factory=list)

@dataclass
class ConsistencyViolation:
    doc_id: str
    expected_value: str
    actual_value: str
    timestamp: float
    consistency_level: str

class CAPMeasurementEngine:
    """Comprehensive CAP metric collection for distributed systems"""
    
    def __init__(self, endpoint: str, key: str, db_name: str, container_name: str):
        self.client = CosmosClient(endpoint, key)
        self.database = self.client.get_database_client(db_name)
        self.container = self.database.get_container_client(container_name)
        
        self.latencies: Dict[str, List[float]] = {
            'create': [], 'read': [], 'update': [], 'delete': []
        }
        self.consistency_violations: List[ConsistencyViolation] = []
        self.operation_count = {'create': 0, 'read': 0, 'update': 0, 'delete': 0}
        self.failed_operations = 0
        self.start_time = None
    
    async def measure_create(self, doc_id: str, partition_key: str, payload: Dict) -> Tuple[bool, float]:
        """Measure CREATE operation latency"""
        start = time.time()
        try:
            doc = {'id': doc_id, 'pk': partition_key, **payload}
            self.container.create_item(body=doc)
            latency = (time.time() - start) * 1000  # ms
            self.latencies['create'].append(latency)
            self.operation_count['create'] += 1
            return True, latency
        except Exception as e:
            logger.error(f"Create failed: {e}")
            self.failed_operations += 1
            return False, 0
    
    async def measure_read(self, doc_id: str, partition_key: str) -> Tuple[bool, float, Optional[Dict]]:
        """Measure READ operation latency + return document"""
        start = time.time()
        try:
            doc = self.container.read_item(item=doc_id, partition_key=partition_key)
            latency = (time.time() - start) * 1000
            self.latencies['read'].append(latency)
            self.operation_count['read'] += 1
            return True, latency, doc
        except Exception as e:
            logger.error(f"Read failed: {e}")
            self.failed_operations += 1
            return False, 0, None
    
    async def measure_update(self, doc_id: str, partition_key: str, updated_payload: Dict) -> Tuple[bool, float]:
        """Measure UPDATE operation latency"""
        start = time.time()
        try:
            doc = self.container.read_item(item=doc_id, partition_key=partition_key)
            doc.update(updated_payload)
            self.container.replace_item(item=doc_id, body=doc)
            latency = (time.time() - start) * 1000
            self.latencies['update'].append(latency)
            self.operation_count['update'] += 1
            return True, latency
        except Exception as e:
            logger.error(f"Update failed: {e}")
            self.failed_operations += 1
            return False, 0
    
    def check_consistency_violation(self, doc_id: str, expected_value: str, 
                                   actual_value: str, consistency_level: str) -> bool:
        """Detect consistency violations"""
        if expected_value != actual_value:
            violation = ConsistencyViolation(
                doc_id=doc_id,
                expected_value=expected_value,
                actual_value=actual_value,
                timestamp=time.time(),
                consistency_level=consistency_level
            )
            self.consistency_violations.append(violation)
            return True
        return False
    
    def calculate_latency_percentiles(self, op_type: str) -> LatencyMetrics:
        """Calculate latency percentiles for an operation type"""
        latencies = self.latencies[op_type]
        if not latencies:
            return LatencyMetrics()
        
        sorted_latencies = sorted(latencies)
        metrics = LatencyMetrics(
            p50=sorted_latencies[int(len(sorted_latencies) * 0.50)],
            p95=sorted_latencies[int(len(sorted_latencies) * 0.95)],
            p99=sorted_latencies[int(len(sorted_latencies) * 0.99)],
            min=min(sorted_latencies),
            max=max(sorted_latencies),
            avg=statistics.mean(latencies),
            stddev=statistics.stdev(latencies) if len(latencies) > 1 else 0,
            all_values=latencies
        )
        return metrics
    
    def get_throughput_metrics(self) -> Dict:
        """Calculate throughput in operations per second"""
        if not self.start_time:
            return {}
        
        duration = time.time() - self.start_time
        total_ops = sum(self.operation_count.values())
        
        return {
            'total_operations': total_ops,
            'total_operations_per_sec': total_ops / duration if duration > 0 else 0,
            'create_ops_per_sec': self.operation_count['create'] / duration if duration > 0 else 0,
            'read_ops_per_sec': self.operation_count['read'] / duration if duration > 0 else 0,
            'update_ops_per_sec': self.operation_count['update'] / duration if duration > 0 else 0,
            'duration_seconds': duration,
            'success_rate': (total_ops - self.failed_operations) / total_ops if total_ops > 0 else 0
        }
    
    def generate_report(self) -> Dict:
        """Generate comprehensive CAP measurement report"""
        report = {
            'timestamp': time.time(),
            'latency_metrics': {}
        }
        
        for op_type in ['create', 'read', 'update', 'delete']:
            metrics = self.calculate_latency_percentiles(op_type)
            report['latency_metrics'][op_type] = {
                'p50_ms': round(metrics.p50, 2),
                'p95_ms': round(metrics.p95, 2),
                'p99_ms': round(metrics.p99, 2),
                'min_ms': round(metrics.min, 2),
                'max_ms': round(metrics.max, 2),
                'avg_ms': round(metrics.avg, 2),
                'stddev_ms': round(metrics.stddev, 2),
                'sample_count': len(metrics.all_values)
            }
        
        report['throughput_metrics'] = self.get_throughput_metrics()
        report['consistency_violations'] = len(self.consistency_violations)
        report['consistency_violation_rate'] = len(self.consistency_violations) / sum(self.operation_count.values()) if sum(self.operation_count.values()) > 0 else 0
        report['operation_count'] = self.operation_count
        report['failed_operations'] = self.failed_operations
        
        return report
    
    def export_metrics_csv(self, filepath: str) -> None:
        """Export all metrics to CSV for analysis"""
        import csv
        with open(filepath, 'w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow(['Operation Type', 'Latency (ms)', 'Timestamp'])
            
            for op_type, latencies in self.latencies.items():
                for lat in latencies:
                    writer.writerow([op_type, lat, time.time()])

# Usage example
if __name__ == "__main__":
    endpoint = os.getenv('COSMOS_ENDPOINT', 'https://cosmos-cap-theorem.documents.azure.com:443/')
    key = os.getenv('COSMOS_KEY', '')
    
    engine = CAPMeasurementEngine(endpoint, key, 'cap-test-db', 'cap-container')
    engine.start_time = time.time()
    
    # Example measurement
    print(json.dumps(engine.generate_report(), indent=2))
