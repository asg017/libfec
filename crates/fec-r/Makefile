# Dev loop for the libfec R package. Run from crates/fec-r.
# Ticket 03 adds `vendor` and `build`.

.PHONY: document install test

document:
	Rscript -e 'devtools::document()'

install:
	R CMD INSTALL .

test:
	Rscript -e 'devtools::test()'
