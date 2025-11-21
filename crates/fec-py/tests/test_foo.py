"""
Tests for libfec_parser.foo module
"""
import pytest
from libfec_parser.foo import bar


class TestBar:
    """Tests for bar function"""
    
    def test_bar_returns_42(self):
        """Test that bar() returns 42"""
        result = bar()
        assert result == 42
    
    def test_bar_returns_int(self):
        """Test that bar() returns an integer"""
        result = bar()
        assert isinstance(result, int)
    
    def test_bar_is_callable(self):
        """Test that bar is callable"""
        assert callable(bar)
    
    def test_bar_no_arguments(self):
        """Test that bar() accepts no arguments"""
        # This should not raise an error
        result = bar()
        assert result is not None
    
    def test_bar_with_arguments_fails(self):
        """Test that bar() with arguments raises TypeError"""
        with pytest.raises(TypeError):
            bar(123)
