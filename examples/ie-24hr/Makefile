tmp.db: a.txt b.txt c.txt
	libfec-dev export FEC-1942895 -o $@
	libfec-dev export a.txt -o $@
	libfec-dev export b.txt -o $@

a.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-01 --received-before 2026-02-07 > $@
b.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-08 --received-before 2026-02-14 > $@
c.txt:
	libfec-dev api --filing-ids-only --form-type F24 --cycle 2026 --received-after 2026-02-15 --received-before 2026-02-21 > $@


rss:
	libfec-dev rss  --watch --form-type F5_24 --interval 30 --export tmp.db 