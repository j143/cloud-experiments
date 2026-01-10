"""Hybrid Consistency Models for CAP Theorem Research

Implements configurable consistency models to test different CAP trade-offs:
- Strong Consistency (CA)
- Eventual Consistency (AP)
- Causal Consistency (hybrid)
- Session Consistency (hybrid)
- Bounded Staleness (hybrid)
"""

import asyncio
import time
import json
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from enum import Enum
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class ConsistencyLevel(Enum):
    """Enum for consistency levels in Cosmos DB"""
    STRONG = "Strong"
    BOUNDED_STALENESS = "BoundedStaleness"
    SESSION = "Session"
    CONSISTENT_PREFIX = "ConsistentPrefix"
    EVENTUAL = "Eventual"


@dataclass
class ConsistencyConfig:
    """Configuration for hybrid consistency experiments"""
    level: ConsistencyLevel
    max_staleness_seconds: Optional[int] = None
    max_staleness_items: Optional[int] = None
    region: str = "North Europe"
    

class HybridConsistencyModel:
    """Implements hybrid consistency models for CAP research"""
    
    def __init__(self, config: ConsistencyConfig):
        self.config = config
        self.consistency_window = self._calculate_window()
        self.replicas = {}
        self.causal_history = {}
        self.session_tokens = {}
        
    def _calculate_window(self) -> Dict[str, Any]:
        """Calculate consistency window based on config"""
        window = {
            'level': self.config.level.value,
            'created_at': datetime.utcnow().isoformat(),
            'max_staleness_seconds': self.config.max_staleness_seconds,
            'max_staleness_items': self.config.max_staleness_items,
        }
        return window
    
    async def write_with_consistency(self, doc_id: str, data: Dict, 
                                     consistency_level: ConsistencyLevel) -> Dict:
        """Write with specified consistency level"""
        timestamp = time.time()
        
        write_result = {
            'doc_id': doc_id,
            'data': data,
            'consistency_level': consistency_level.value,
            'write_timestamp': timestamp,
            'acknowledged': False,
            'replicas_acked': 0,
        }
        
        if consistency_level == ConsistencyLevel.STRONG:
            # Wait for all replicas
            write_result['replicas_acked'] = 3  # Simulated
            write_result['acknowledged'] = True
            write_result['latency_ms'] = 150  # Higher for strong consistency
            
        elif consistency_level == ConsistencyLevel.EVENTUAL:
            # Fire and forget
            write_result['replicas_acked'] = 1
            write_result['acknowledged'] = True
            write_result['latency_ms'] = 20  # Lower latency
            
        elif consistency_level == ConsistencyLevel.SESSION:
            # Wait for session replica
            write_result['replicas_acked'] = 2
            write_result['acknowledged'] = True
            write_result['session_token'] = f"token_{doc_id}_{timestamp}"
            write_result['latency_ms'] = 80
            self.session_tokens[doc_id] = write_result['session_token']
            
        elif consistency_level == ConsistencyLevel.BOUNDED_STALENESS:
            # Wait up to max staleness
            write_result['replicas_acked'] = 3
            write_result['acknowledged'] = True
            write_result['latency_ms'] = 120
            
        elif consistency_level == ConsistencyLevel.CONSISTENT_PREFIX:
            # Causal consistency
            write_result['replicas_acked'] = 2
            write_result['acknowledged'] = True
            write_result['causal_token'] = f"causal_{doc_id}_{timestamp}"
            write_result['latency_ms'] = 90
            self.causal_history[doc_id] = write_result['causal_token']
        
        logger.info(f"Write with {consistency_level.value}: {doc_id} replicas_acked={write_result['replicas_acked']}")
        return write_result
    
    async def read_with_consistency(self, doc_id: str, 
                                    consistency_level: ConsistencyLevel,
                                    session_token: Optional[str] = None) -> Dict:
        """Read with specified consistency level"""
        timestamp = time.time()
        
        read_result = {
            'doc_id': doc_id,
            'consistency_level': consistency_level.value,
            'read_timestamp': timestamp,
            'found': True,
            'staleness_ms': 0,
        }
        
        if consistency_level == ConsistencyLevel.STRONG:
            # Read from primary replica
            read_result['staleness_ms'] = 0
            read_result['latency_ms'] = 100
            
        elif consistency_level == ConsistencyLevel.EVENTUAL:
            # Read from any replica - may be stale
            read_result['staleness_ms'] = 500  # Simulated staleness
            read_result['latency_ms'] = 10
            
        elif consistency_level == ConsistencyLevel.SESSION:
            if session_token and session_token == self.session_tokens.get(doc_id):
                read_result['staleness_ms'] = 0
                read_result['latency_ms'] = 80
            else:
                read_result['staleness_ms'] = 100
                read_result['latency_ms'] = 90
                
        elif consistency_level == ConsistencyLevel.BOUNDED_STALENESS:
            read_result['staleness_ms'] = 200  # Within bound
            read_result['latency_ms'] = 110
            
        elif consistency_level == ConsistencyLevel.CONSISTENT_PREFIX:
            if doc_id in self.causal_history:
                read_result['staleness_ms'] = 50
                read_result['latency_ms'] = 85
            else:
                read_result['staleness_ms'] = 0
                read_result['latency_ms'] = 95
        
        logger.info(f"Read with {consistency_level.value}: {doc_id} staleness={read_result['staleness_ms']}ms")
        return read_result
    
    def get_consistency_metrics(self) -> Dict[str, Any]:
        """Get metrics for this consistency model"""
        return {
            'consistency_level': self.config.level.value,
            'window': self.consistency_window,
            'region': self.config.region,
            'timestamp': datetime.utcnow().isoformat(),
        }


class MultiRegionConsistency:
    """Multi-region consistency coordination"""
    
    def __init__(self, regions: List[str]):
        self.regions = regions
        self.region_models: Dict[str, HybridConsistencyModel] = {}
        self.cross_region_latencies = {}  # ms
        
    def add_region(self, region: str, consistency_level: ConsistencyLevel):
        """Add a region with specified consistency"""
        config = ConsistencyConfig(level=consistency_level, region=region)
        self.region_models[region] = HybridConsistencyModel(config)
        logger.info(f"Added region {region} with {consistency_level.value}")
    
    async def replicate_across_regions(self, doc_id: str, data: Dict) -> Dict:
        """Replicate write across multiple regions"""
        replication_result = {
            'doc_id': doc_id,
            'regions': self.regions,
            'replicated_regions': [],
            'failed_regions': [],
            'replication_time_ms': 0,
        }
        
        start_time = time.time()
        
        for region in self.regions:
            try:
                # Simulate replication with inter-region latency
                region_latency = self.cross_region_latencies.get(
                    region, 100  # Default 100ms inter-region latency
                )
                await asyncio.sleep(region_latency / 1000)
                replication_result['replicated_regions'].append(region)
            except Exception as e:
                replication_result['failed_regions'].append(region)
                logger.error(f"Replication to {region} failed: {e}")
        
        replication_result['replication_time_ms'] = (time.time() - start_time) * 1000
        return replication_result


async def main():
    """Run hybrid consistency model tests"""
    logger.info("=" * 60)
    logger.info("Hybrid Consistency Model Tests")
    logger.info("=" * 60)
    
    # Test different consistency levels
    levels = [
        ConsistencyLevel.STRONG,
        ConsistencyLevel.BOUNDED_STALENESS,
        ConsistencyLevel.SESSION,
        ConsistencyLevel.CONSISTENT_PREFIX,
        ConsistencyLevel.EVENTUAL,
    ]
    
    for level in levels:
        logger.info(f"\nTesting {level.value} Consistency")
        config = ConsistencyConfig(level=level, max_staleness_seconds=5)
        model = HybridConsistencyModel(config)
        
        # Test write
        write_result = await model.write_with_consistency(
            f"doc_{level.value}",
            {"test": "data"},
            level
        )
        logger.info(f"Write latency: {write_result['latency_ms']}ms, Replicas: {write_result['replicas_acked']}")
        
        # Test read
        read_result = await model.read_with_consistency(
            f"doc_{level.value}",
            level
        )
        logger.info(f"Read latency: {read_result['latency_ms']}ms, Staleness: {read_result['staleness_ms']}ms")
    
    # Test multi-region
    logger.info("\n" + "=" * 60)
    logger.info("Multi-Region Replication Test")
    logger.info("=" * 60)
    
    multi_region = MultiRegionConsistency(["North Europe", "East US", "Southeast Asia"])
    multi_region.add_region("North Europe", ConsistencyLevel.STRONG)
    multi_region.add_region("East US", ConsistencyLevel.EVENTUAL)
    multi_region.add_region("Southeast Asia", ConsistencyLevel.SESSION)
    
    replication_result = await multi_region.replicate_across_regions(
        "multi_region_doc",
        {"data": "test"}
    )
    logger.info(f"Replication completed: {replication_result}")


if __name__ == "__main__":
    asyncio.run(main())
