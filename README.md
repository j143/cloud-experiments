# cloud-experiments

Experiment with Cloud platform resources to understand their architecture

## Overview

This repository contains experiments for testing and understanding the CAP theorem using Azure Cosmos DB. It includes:

- CAP theorem verification scripts
- Load generation and measurement tools
- Hybrid consistency model implementations
- Automated testing with GitHub Actions
- Azure deployment automation

## Features

### Python Scripts

- **cap_test_simple.py**: Basic CAP theorem tests with different consistency levels
- **cap_measurement.py**: Comprehensive CAP metric collection engine
- **load_generator.py**: Production-grade workload generator
- **hybrid_consistency.py**: Hybrid consistency model implementations
- **run_all_experiments.py**: Master orchestration script for all experiments

### GitHub Actions Workflows

#### 1. Tests Workflow (CI)

Automatically runs on push and pull requests to `main`, `master`, or `develop` branches.

**What it does:**
- Tests against Python 3.9, 3.10, 3.11, and 3.12
- Validates Python syntax for all scripts
- Runs unit tests with pytest
- Ensures code quality and functionality

#### 2. Azure Deployment Workflow

Manual workflow (`workflow_dispatch`) for deploying Azure Cosmos DB infrastructure.

**How to use:**
1. Go to Actions tab in GitHub
2. Select "Deploy to Azure" workflow
3. Click "Run workflow"
4. Configure deployment options:
   - Resource Group name
   - Cosmos DB account name (must be globally unique)
   - Database and container names
   - Primary and secondary regions
   - Throughput (RU/s)
   - Consistency level (Strong, BoundedStaleness, Session, etc.)
   - Enable/disable multi-region writes

**Prerequisites:**
- Set up `AZURE_CREDENTIALS` secret in repository settings
- Azure subscription with appropriate permissions

## Getting Started

### Installation

```bash
# Clone the repository
git clone https://github.com/j143/cloud-experiments.git
cd cloud-experiments

# Install dependencies
pip install -r requirements.txt
```

### Running Tests

```bash
# Run all tests
pytest tests/ -v

# Run specific test file
pytest tests/test_cap_scripts.py -v

# Run with coverage
pytest tests/ --cov
```

### Running CAP Experiments

**Prerequisites:**
- Azure Cosmos DB account (use GitHub Actions or setup-cosmos-db.sh)
- Set environment variables:

```bash
export COSMOS_ENDPOINT='https://your-account.documents.azure.com:443/'
export COSMOS_KEY='your-cosmos-key'
```

**Run individual tests:**

```bash
# Basic CAP theorem tests
python cap_test_simple.py

# All experiments
python run_all_experiments.py
```

### Manual Azure Deployment (CLI)

```bash
# Run the setup script
chmod +x setup-cosmos-db.sh
./setup-cosmos-db.sh
```

## Project Structure

```
.
├── .github/
│   └── workflows/
│       ├── test.yml              # CI testing workflow
│       └── deploy-azure.yml      # Azure deployment workflow
├── tests/
│   ├── __init__.py
│   ├── conftest.py
│   └── test_cap_scripts.py       # Unit tests
├── azure/                         # Azure-specific configurations
├── cap_test_simple.py             # Basic CAP tests
├── cap_measurement.py             # Metrics collection
├── load_generator.py              # Load testing
├── hybrid_consistency.py          # Consistency models
├── run_all_experiments.py         # Orchestration script
├── setup-cosmos-db.sh             # Azure setup script
└── requirements.txt               # Python dependencies
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `pytest tests/ -v`
5. Submit a pull request

## License

This project is for educational and experimental purposes.

