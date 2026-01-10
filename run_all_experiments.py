#!/usr/bin/env python3
"""Master Orchestration Script for CAP Theorem Research on Azure

Runs all experiments sequentially and logs results:
- CAP Theorem tests
- Load generator benchmark
- Consistency measurement
- Hybrid consistency models
- Multi-region replication

All output to CSV and JSON for analysis
"""

import subprocess
import sys
import json
import logging
import os
from datetime import datetime
from pathlib import Path

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

OUTPUT_DIR = Path('results')
OUTPUT_DIR.mkdir(exist_ok=True)

def run_command(cmd, description):
    """Execute command and log results"""
    logger.info(f"\n{'='*70}")
    logger.info(f"RUNNING: {description}")
    logger.info(f"Command: {' '.join(cmd)}")
    logger.info(f"{'='*70}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        logger.info(result.stdout)
        if result.returncode != 0:
            logger.error(f"ERROR: {result.stderr}")
            return False
        return True
    except subprocess.TimeoutExpired:
        logger.error(f"TIMEOUT: {description} took too long")
        return False
    except Exception as e:
        logger.error(f"EXCEPTION: {e}")
        return False

def main():
    """Execute all CAP theorem experiments"""
    logger.info("\n" + "#"*70)
    logger.info("# CAP THEOREM RESEARCH SUITE - STARTING")
    logger.info(f"# Started: {datetime.utcnow().isoformat()}")
    logger.info("#"*70)
    
    results = {}
    
    # Test 1: Simple CAP test
    results['cap_test'] = run_command(
        [sys.executable, 'cap_test_simple.py'],
        'CAP Theorem Basic Tests'
    )
    
    # Test 2: Load generator
    results['load_gen'] = run_command(
        [sys.executable, 'load_generator.py'],
        'Production-Grade Load Generator'
    )
    
    # Test 3: CAP measurement engine
    results['cap_measurement'] = run_command(
        [sys.executable, 'cap_measurement.py'],
        'CAP Theorem Measurement Engine'
    )
    
    # Test 4: Hybrid consistency models
    results['hybrid_consistency'] = run_command(
        [sys.executable, 'hybrid_consistency.py'],
        'Hybrid Consistency Models'
    )
    
    # Azure CLI tests: Check Cosmos DB setup
    logger.info("\n" + "="*70)
    logger.info("VALIDATING AZURE COSMOS DB SETUP")
    logger.info("="*70)
    
    try:
        check_cosmos = subprocess.run(
            ['az', 'cosmosdb', 'list', '--query', '[].name'],
            capture_output=True, text=True
        )
        logger.info(f"Cosmos DB Accounts: {check_cosmos.stdout}")
    except Exception as e:
        logger.warning(f"Azure CLI check failed: {e}")
    
    # Summary
    logger.info("\n" + "#"*70)
    logger.info("# EXECUTION SUMMARY")
    logger.info("#"*70)
    for test_name, success in results.items():
        status = "✓ PASS" if success else "✗ FAIL"
        logger.info(f"{status} - {test_name}")
    
    total_tests = len(results)
    passed_tests = sum(1 for v in results.values() if v)
    logger.info(f"\nTotal: {passed_tests}/{total_tests} tests passed")
    logger.info(f"Ended: {datetime.utcnow().isoformat()}")
    logger.info("#"*70 + "\n")
    
    # Save results
    results_file = OUTPUT_DIR / f"experiment_results_{datetime.utcnow().isoformat()}.json"
    with open(results_file, 'w') as f:
        json.dump(results, f, indent=2)
    logger.info(f"Results saved to: {results_file}")
    
    return 0 if all(results.values()) else 1

if __name__ == '__main__':
    sys.exit(main())
