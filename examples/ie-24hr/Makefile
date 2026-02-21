tmp.db: schema.sql a.txt b.txt c.txt
	libfec-dev export FEC-1942895 -o $@
	solite-dev run schema.sql $@
	libfec-dev export a.txt -o $@
	libfec-dev export b.txt -o $@
	
procedures.sql.json: procedures.sql tmp.db
	solite-dev codegen --schema tmp.db procedures.sql > $@

generated_client.py: procedures.sql.json
	uv run solite-codegen-python-sqlite3 < $< > $@

a.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-01 --received-before 2026-02-07 > $@
b.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-08 --received-before 2026-02-14 > $@
c.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-15 --received-before 2026-02-21 > $@