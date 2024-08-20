#cat fec-parser-macros/src/mappings2.json | rg '^\s+".*",?$' | sort | uniq

column_names.txt: src/mappings2.json
	cat $< | rg '^\s+".*",?$$' | awk -F'"' '/"/ {print $$2}' | sort | uniq > $@


date_columns.txt: column_names.txt
	cat $< | rg '(^date_)|(_date$$)' > $@
