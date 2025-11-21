# uv run --no-project --isolated --with 'fec_py @ file://../../dist/fec_py-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' demo.py <file1.fec> <file2.fec> ...

from fec_py._core import hello_from_bin
from pathlib import Path
import sys

def main() -> None:
    # Get file paths from command line arguments
    if len(sys.argv) < 2:
        print("Usage: demo.py <fec_file1> [fec_file2] [fec_file3] ...")
        print("Example: demo.py 1887722.fec")
        sys.exit(1)
    
    fec_files = sys.argv[1:]
    
    for fec_file in fec_files:
        file_path = Path(fec_file)
        result = hello_from_bin(file_path.read_bytes())
        print(result)
        
if __name__ == "__main__":
    main()