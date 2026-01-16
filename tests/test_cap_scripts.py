"""
Unit tests for CAP theorem verification scripts
Tests basic functionality without requiring Azure credentials
"""

import pytest
import sys
import os
from unittest.mock import Mock, patch, MagicMock


class TestCapTestSimple:
    """Tests for cap_test_simple.py"""
    
    @patch('cap_test_simple.COSMOS_KEY', 'test-key')
    @patch('cap_test_simple.CosmosClient')
    def test_init_client_with_key(self, mock_cosmos_client):
        """Test that client initializes with valid key"""
        from cap_test_simple import init_client
        
        client = init_client()
        mock_cosmos_client.assert_called_once()
        assert client is not None
    
    @patch('cap_test_simple.COSMOS_KEY', '')
    def test_init_client_without_key(self):
        """Test that client exits without key"""
        from cap_test_simple import init_client
        
        with pytest.raises(SystemExit):
            init_client()
    
    @patch('cap_test_simple.init_client')
    def test_strong_consistency_success(self, mock_init_client):
        """Test strong consistency test succeeds with proper mocks"""
        from cap_test_simple import test_strong_consistency
        
        # Setup mocks
        mock_container = Mock()
        mock_container.create_item.return_value = {'id': 'test-id', 'value': 'initial'}
        mock_container.read_item.return_value = {'id': 'test-id', 'value': 'initial'}
        mock_container.replace_item.return_value = {'id': 'test-id', 'value': 'updated'}
        
        mock_db = Mock()
        mock_db.get_container_client.return_value = mock_container
        
        mock_client = Mock()
        mock_client.get_database_client.return_value = mock_db
        mock_init_client.return_value = mock_client
        
        result = test_strong_consistency()
        assert result is True
    
    @patch('cap_test_simple.init_client')
    def test_eventual_consistency_success(self, mock_init_client):
        """Test eventual consistency test succeeds with proper mocks"""
        from cap_test_simple import test_eventual_consistency
        
        # Setup mocks
        mock_container = Mock()
        mock_container.create_item.return_value = {'id': 'test-id', 'version': 1}
        mock_container.replace_item.return_value = {'id': 'test-id', 'version': 4}
        mock_container.read_item.return_value = {'id': 'test-id', 'version': 4}
        
        mock_db = Mock()
        mock_db.get_container_client.return_value = mock_container
        
        mock_client = Mock()
        mock_client.get_database_client.return_value = mock_db
        mock_init_client.return_value = mock_client
        
        result = test_eventual_consistency()
        assert result is True
    
    @patch('cap_test_simple.init_client')
    def test_partition_tolerance_success(self, mock_init_client):
        """Test partition tolerance test succeeds with proper mocks"""
        from cap_test_simple import test_partition_tolerance
        
        # Setup mocks
        mock_container = Mock()
        mock_container.create_item.return_value = {'id': 'test-id'}
        mock_container.query_items.return_value = [{'id': f'doc-{i}'} for i in range(10)]
        
        mock_db = Mock()
        mock_db.get_container_client.return_value = mock_container
        
        mock_client = Mock()
        mock_client.get_database_client.return_value = mock_db
        mock_init_client.return_value = mock_client
        
        result = test_partition_tolerance()
        assert result is True


class TestImports:
    """Test that all modules can be imported successfully"""
    
    def test_import_cap_test_simple(self):
        """Test cap_test_simple can be imported"""
        import cap_test_simple
        assert hasattr(cap_test_simple, 'main')
        assert hasattr(cap_test_simple, 'test_strong_consistency')
        assert hasattr(cap_test_simple, 'test_eventual_consistency')
        assert hasattr(cap_test_simple, 'test_partition_tolerance')
    
    def test_import_run_all_experiments(self):
        """Test run_all_experiments can be imported"""
        import run_all_experiments
        assert hasattr(run_all_experiments, 'main')
        assert hasattr(run_all_experiments, 'run_command')
    
    def test_import_cap_measurement(self):
        """Test cap_measurement can be imported"""
        import cap_measurement
        assert hasattr(cap_measurement, 'CAPMeasurementEngine')
        assert hasattr(cap_measurement, 'LatencyMetrics')
        assert hasattr(cap_measurement, 'ConsistencyViolation')
    
    def test_import_load_generator(self):
        """Test load_generator can be imported"""
        import load_generator
        assert hasattr(load_generator, 'WorkloadGenerator')
        assert hasattr(load_generator, 'LoadConfig')
        assert hasattr(load_generator, 'LoadProfile')
    
    def test_import_hybrid_consistency(self):
        """Test hybrid_consistency can be imported"""
        import hybrid_consistency
        assert hasattr(hybrid_consistency, 'HybridConsistencyModel')
        assert hasattr(hybrid_consistency, 'ConsistencyLevel')
        assert hasattr(hybrid_consistency, 'ConsistencyConfig')


class TestLoadGenerator:
    """Tests for load_generator.py"""
    
    def test_load_config_defaults(self):
        """Test LoadConfig has proper defaults"""
        from load_generator import LoadConfig
        
        config = LoadConfig()
        assert config.target_ops_per_sec == 100
        assert config.duration_seconds == 60
        assert config.read_write_ratio == 0.8
        assert config.num_threads == 10
    
    def test_operation_type_enum(self):
        """Test OperationType enum values"""
        from load_generator import OperationType
        
        assert OperationType.CREATE.value == "create"
        assert OperationType.READ.value == "read"
        assert OperationType.UPDATE.value == "update"
        assert OperationType.DELETE.value == "delete"
    
    def test_load_profile_enum(self):
        """Test LoadProfile enum values"""
        from load_generator import LoadProfile
        
        assert LoadProfile.CONSTANT.value == "constant"
        assert LoadProfile.RAMP_UP.value == "ramp_up"
        assert LoadProfile.BURST.value == "burst"
        assert LoadProfile.REALISTIC.value == "realistic"


class TestCAPMeasurement:
    """Tests for cap_measurement.py"""
    
    def test_latency_metrics_dataclass(self):
        """Test LatencyMetrics dataclass"""
        from cap_measurement import LatencyMetrics
        
        metrics = LatencyMetrics()
        assert metrics.p50 == 0.0
        assert metrics.p95 == 0.0
        assert metrics.p99 == 0.0
        assert isinstance(metrics.all_values, list)
    
    def test_consistency_violation_dataclass(self):
        """Test ConsistencyViolation dataclass"""
        from cap_measurement import ConsistencyViolation
        
        violation = ConsistencyViolation(
            doc_id="test-doc",
            expected_value="expected",
            actual_value="actual",
            timestamp=123456.789,
            consistency_level="Strong"
        )
        assert violation.doc_id == "test-doc"
        assert violation.expected_value == "expected"
        assert violation.actual_value == "actual"
        assert violation.timestamp == 123456.789


class TestHybridConsistency:
    """Tests for hybrid_consistency.py"""
    
    def test_consistency_level_enum(self):
        """Test ConsistencyLevel enum values"""
        from hybrid_consistency import ConsistencyLevel
        
        assert ConsistencyLevel.STRONG.value == "Strong"
        assert ConsistencyLevel.EVENTUAL.value == "Eventual"
        assert ConsistencyLevel.SESSION.value == "Session"
        assert ConsistencyLevel.BOUNDED_STALENESS.value == "BoundedStaleness"
        assert ConsistencyLevel.CONSISTENT_PREFIX.value == "ConsistentPrefix"
    
    def test_consistency_config_dataclass(self):
        """Test ConsistencyConfig dataclass"""
        from hybrid_consistency import ConsistencyConfig, ConsistencyLevel
        
        config = ConsistencyConfig(
            level=ConsistencyLevel.STRONG,
            region="East US"
        )
        assert config.level == ConsistencyLevel.STRONG
        assert config.region == "East US"
        assert config.max_staleness_seconds is None
