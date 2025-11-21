#!/usr/bin/env python3
"""
Demo script for libfec_parser.fecfile - a compatibility layer for the fecfile PyPI package.

This demonstrates the API that mimics the original fecfile library:
- loads() - Parse FEC content from string/bytes
- from_file() - Load from a file path
- from_http() - Load from FEC website (if available)
- parse_header() - Parse just the header
- parse_line() - Parse a single line
- print_example() - Print a sample of the parsed data

Usage:
    python demo-fecfile.py <file1.fec> [file2.fec] ...
"""

import sys
from pathlib import Path
from libfec_parser.fecfile import loads, from_file, parse_header, parse_line, print_example

def demo_loads(file_path):
    """Demo the loads() function"""
    print(f"\n{'='*70}")
    print(f"Demo: loads() with file: {file_path}")
    print('='*70)
    
    # Read file content
    content = Path(file_path).read_text(encoding='utf-8', errors='ignore')
    
    # Parse with loads()
    parsed = loads(content)
    
    print(f"\n📋 Header:")
    print(f"  FEC Version: {parsed['header']['fec_version']}")
    print(f"  Software: {parsed['header']['software_name']} v{parsed['header']['software_version']}")
    
    print(f"\n📄 Filing:")
    print(f"  Form Type: {parsed['filing']['form_type']}")
    print(f"  Filer ID: {parsed['filing'].get('filer_committee_id_number', 'N/A')}")
    if 'committee_name' in parsed['filing']:
        print(f"  Committee: {parsed['filing']['committee_name']}")
    
    print(f"\n📊 Itemizations:")
    for schedule, items in parsed['itemizations'].items():
        print(f"  {schedule}: {len(items)} items")
    
    if parsed['text']:
        print(f"\n📝 Text records: {len(parsed['text'])} items")
    
    return parsed

def demo_from_file(file_path):
    """Demo the from_file() function"""
    print(f"\n{'='*70}")
    print(f"Demo: from_file() with: {file_path}")
    print('='*70)
    
    parsed = from_file(file_path)
    
    print(f"\n📋 Header:")
    print(f"  FEC Version: {parsed['header']['fec_version']}")
    
    print(f"\n📄 Filing:")
    print(f"  Form Type: {parsed['filing']['form_type']}")
    
    print(f"\n📊 Itemizations summary:")
    total_items = sum(len(items) for items in parsed['itemizations'].values())
    print(f"  Total itemization records: {total_items}")
    
    return parsed

def demo_filter_itemizations(file_path):
    """Demo the filter_itemizations option"""
    print(f"\n{'='*70}")
    print(f"Demo: loads() with filter_itemizations=['SA', 'SB']")
    print('='*70)
    
    content = Path(file_path).read_text(encoding='utf-8', errors='ignore')
    
    # Parse with filter
    parsed = loads(content, options={'filter_itemizations': ['SA', 'SB']})
    
    print(f"\n📊 Filtered Itemizations (only SA and SB schedules):")
    for schedule, items in parsed['itemizations'].items():
        print(f"  {schedule}: {len(items)} items")

def demo_parse_header(file_path):
    """Demo the parse_header() function"""
    print(f"\n{'='*70}")
    print(f"Demo: parse_header()")
    print('='*70)
    
    # Read just the first line (header)
    with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
        header_line = f.readline()
    
    header, version, lines_consumed = parse_header(header_line)
    
    print(f"\n📋 Parsed Header:")
    print(f"  Version: {version}")
    print(f"  Lines consumed: {lines_consumed}")
    print(f"  Record type: {header['record_type']}")
    print(f"  Software: {header['software_name']} v{header['software_version']}")

def demo_parse_line(file_path):
    """Demo the parse_line() function"""
    print(f"\n{'='*70}")
    print(f"Demo: parse_line()")
    print('='*70)
    
    # Read first few lines
    with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
        lines = [f.readline() for _ in range(3)]
    
    # Parse header to get version
    header, version, _ = parse_header(lines[0])
    
    # Parse the cover line (second line)
    cover = parse_line(lines[1], version, line_num=2)
    
    print(f"\n📄 Parsed Cover Line (line 2):")
    print(f"  Form type: {cover.get('form_type', 'N/A')}")
    print(f"  Fields in cover: {len(cover)}")
    print(f"  Sample fields: {list(cover.keys())[:5]}...")

def demo_print_example(file_path):
    """Demo the print_example() function"""
    print(f"\n{'='*70}")
    print(f"Demo: print_example()")
    print('='*70)
    
    parsed = from_file(file_path)
    
    print("\n📋 Example output (first item of each type):")
    print_example(parsed)

def main():    
    fec_files = sys.argv[1:]
    
    # Run demos on the first file
    first_file = fec_files[0]
    
    print("\n" + "="*70)
    print(f"FEC FILE COMPATIBILITY API DEMO")
    print(f"File: {first_file}")
    print("="*70)
    
    # Demo each function
    try:
        demo_from_file(first_file)
        demo_loads(first_file)
        demo_parse_header(first_file)
        demo_parse_line(first_file)
        demo_filter_itemizations(first_file)
        demo_print_example(first_file)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
    
    # Process remaining files with brief output
    if len(fec_files) > 1:
        print(f"\n{'='*70}")
        print(f"Processing remaining {len(fec_files) - 1} files...")
        print('='*70)
        
        for fec_file in fec_files[1:]:
            try:
                parsed = from_file(fec_file)
                total = sum(len(items) for items in parsed['itemizations'].values())
                print(f"\n✓ {Path(fec_file).name}: "
                      f"v{parsed['header']['fec_version']}, "
                      f"{parsed['filing']['form_type']}, "
                      f"{total} itemizations")
            except Exception as e:
                print(f"\n✗ {Path(fec_file).name}: Error - {e}")
    
    print(f"\n{'='*70}")
    print("✨ Demo complete!")
    print('='*70)

if __name__ == "__main__":
    main()
