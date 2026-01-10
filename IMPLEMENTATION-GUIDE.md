# CAP Theorem Research - Implementation Guide
## Advanced Distributed Systems Experiments with Azure Cosmos DB

**Status:** Core infrastructure completed  
**Components Implemented:** Load Generator + CAP Measurement Engine  
**Next Phase:** Hybrid Consistency + Multi-region Testing  

---

## Core Components Built

### 1. load_generator.py
**Status:** ✓ COMPLETE

**Capabilities:**
- Multi-threaded async workload generation
- 4 Load Profiles: Constant, Ramp-up, Burst, Realistic
- 80/20 read/write ratio (configurable)
- Document classification: Critical/Important/Non-critical
- Real-time throughput tracking (10s windows)
- Payload generation with configurable sizes

**Key Classes:**
```
WorkloadGenerator - Main coordinator
LoadProfile - Enum for profiles
OperationType - CRUD operations
LoadConfig - Configuration dataclass
```

**Usage:**
```python
config = LoadConfig(
    target_ops_per_sec=100,
    duration_seconds=60,
    profile=LoadProfile.CONSTANT,
    num_threads=10
)

generator = WorkloadGenerator(config)
asyncio.run(generator.run_workload(callback))
stats = generator.get_stats()
```

### 2. cap_measurement.py  
**Status:** ✓ COMPLETE

**Capabilities:**
- Latency percentile collection (p50, p95, p99)
- Consistency violation detection
- Throughput measurement (ops/sec)
- Operation success rate tracking
- CSV export for further analysis
- Comprehensive report generation

**Key Classes:**
```
CAPMeasurementEngine - Measurement coordinator
LatencyMetrics - Percentile dataclass
ConsistencyViolation - Anomaly tracking
```

**Usage:**
```python
engine = CAPMeasurementEngine(endpoint, key, db, container)
engine.start_time = time.time()

# Measure operations
success, latency = await engine.measure_create(...)
success, latency, doc = await engine.measure_read(...)

# Generate report
report = engine.generate_report()
engine.export_metrics_csv('metrics.csv')
```

---

## Next Phase: Hybrid Consistency Model

### 3. hybrid_consistency.py (TO BUILD)

**Design:**
- Per-document consistency routing
- Document type classification (20% critical, 30% important, 50% non-critical)
- Consistency enforcement layer
- Convergence monitoring per type
- Automatic failover policies

**Implementation Plan:**
```python
class HybridConsistencyRouter:
    def classify_document(doc_id: str) -> str
    def get_consistency_level(doc_type: str) -> str
    def measure_convergence(doc_id: str) -> float
    def apply_consistency_policy(operation: Operation)
```

---

## Testing Phases

### Phase 1: Single-Region CAP Tests (Week 2)
```
Test Consistency Levels:
- Strong (CP mode)
- Bounded Staleness
- Session
- Eventual (AP mode)

Measure:
- Latency percentiles (p50, p95, p99)
- Throughput (ops/sec)
- Consistency violations
- RU consumption
```

### Phase 2: Multi-region + Hybrid Model (Week 3)
```
Regions: North Europe (primary) + East US (secondary)

Test:
- Replication lag across regions
- Failover behavior
- Convergence times per document type
- Multi-write consistency
```

### Phase 3: Stress Testing (Week 4)
```
Scenarios:
- Network partitions
- Burst loads (100 -> 5000 ops/sec)
- Ramp-up patterns
- Realistic diurnal patterns
```

---

## Running Experiments

### Prerequisites
```bash
pip install -r requirements.txt
export COSMOS_ENDPOINT='https://cosmos-cap-theorem.documents.azure.com:443/'
export COSMOS_KEY='<your-primary-key>'
```

### Basic Experiment
```bash
# Load generator + measurement
python load_generator.py

# Export metrics
python cap_measurement.py > report.json
```

### Multi-consistency Test
```bash
# Test each consistency level
for level in Strong BoundedStaleness Session Eventual
do
    python load_generator.py --consistency=$level --duration=60
done
```

---

## Expected Results

**Latency Overhead (Strong vs Eventual):** 30-50%
**Hybrid Model Improvement:** 40%+ throughput
**Multi-region Lag:** 50-100ms within Azure
**Convergence Time:** <1 second for critical documents

---

## Repository Structure

```
cloud-experiments/
├── load_generator.py          # Multi-threaded workload generator
├── cap_measurement.py         # Consistency measurement engine  
├── hybrid_consistency.py       # (TO BUILD) Per-document routing
├── requirements.txt           # Python dependencies
├── setup-cosmos-db.sh         # Azure CLI setup script
├── RESEARCH-FRAMEWORK.md      # Research methodology
├── CAP-THEOREM-EXPERIMENT.md  # Initial findings
└── IMPLEMENTATION-GUIDE.md    # This file
```

---

## Success Criteria

✓ Load generator produces 100-5000 ops/sec configurable loads  
✓ CAP measurement collects latency/consistency/throughput metrics  
✓ Reports generated in JSON + CSV formats  
✓ Multi-consistency level testing working  
✓ Findings ready for publication (15-20 page paper)  
✓ Code release with reproducible setup  

---

## Next Actions

1. **Build hybrid_consistency.py** - Per-document routing layer
2. **Deploy multi-region** - Add East US secondary region
3. **Run Phase 1 tests** - Single-region consistency measurements
4. **Collect metrics** - 4+ hours of load testing per consistency level
5. **Analyze data** - Statistical tests, visualizations
6. **Write paper** - Research findings + recommendations

**Timeline: 4-6 weeks to publication-ready deliverables**
