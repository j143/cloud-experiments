"""
Pytest configuration for CAP theorem tests
"""

import sys
import os

# Add the parent directory to the Python path so tests can import modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
