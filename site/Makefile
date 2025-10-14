reference/filings.sql:
	solite-dev q ca01.db \
		"select sql from sqlite_master where type = 'table' and tbl_name = 'libfec_filings';" \
		-f value \
		-o $@
	
reference/cli-help.txt:
	../target/debug/libfec --help > $@

reference/cli-export-help.txt:
	../target/debug/libfec export --help > $@

reference/cli-info-help.txt:
	../target/debug/libfec info --help > $@

reference/cli-fastfec-help.txt:
	../target/debug/libfec fastfec --help > $@


sql: reference/filings.sql
cli: reference/cli-help.txt reference/cli-export-help.txt reference/cli-info-help.txt reference/cli-fastfec-help.txt

all: cli sql