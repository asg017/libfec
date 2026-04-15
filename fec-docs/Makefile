XLSX_SRC := eFilingFormats/FEC_v8x/FEC_Format_v8.4.xlsx
BASE_URL := https://www.fec.gov/resources/cms-content/documents/policy-guidance

FORMS := F1 F1M F2 F3 F3P F3L F3X F4 F5 F6 F7 F8 F9 F13

.PHONY: all split $(FORMS)

all: split $(FORMS)

# Split worksheets into forms/*/format.xlsx
split: $(XLSX_SRC) split_worksheets.py
	uv run split_worksheets.py split $< forms
	@touch $@

# Generic PDF download rule
forms/%.pdf:
	@mkdir -p $(dir $@)
	curl -fsSL -o $@ $(BASE_URL)/$(notdir $@)

# Extract raw text from PDF
forms/%.raw.txt: forms/%.pdf split_worksheets.py
	uv run split_worksheets.py extract-text $<

# Clean raw text into readable form
forms/%.txt: forms/%.raw.txt split_worksheets.py
	uv run split_worksheets.py clean-text $<

# --- Form targets ---

F1: forms/F1/fecfrm1.pdf forms/F1/fecfrm1i.pdf forms/F1/fecfrm1i.txt

F1M: forms/F1M/fecfrm1m.pdf forms/F1M/fecfrm1mi.pdf forms/F1M/fecfrm1mi.txt

F2: forms/F2/fecfrm2.pdf forms/F2/fecfrm2i.pdf forms/F2/fecfrm2i.txt

F3: forms/F3/fecfrm3.pdf forms/F3/fecfrm3i.pdf forms/F3/fecfrm3i.txt \
    forms/F3/fecfrm3post.pdf forms/F3/fecfrm3posti.pdf forms/F3/fecfrm3posti.txt

F3P: forms/F3P/fecfrm3p.pdf forms/F3P/fecfrm3pi.pdf forms/F3P/fecfrm3pi.txt \
     forms/F3P/fecfrm3ppost.pdf forms/F3P/fecfrm3pposti.pdf forms/F3P/fecfrm3pposti.txt

F3L: forms/F3L/fecfrm3l.pdf forms/F3L/fecfrm3li.pdf forms/F3L/fecfrm3li.txt

F3X: forms/F3X/fecfrm3x.pdf forms/F3X/fecfrm3xi.pdf forms/F3X/fecfrm3xi.txt \
     forms/F3X/fecfrm3xe.pdf forms/F3X/fecfrm3xei.pdf forms/F3X/fecfrm3xei.txt

F4: forms/F4/fecfrm4.pdf forms/F4/fecfrm4i.pdf forms/F4/fecfrm4i.txt

F5: forms/F5/fecfrm5.pdf forms/F5/fecfrm5i.pdf forms/F5/fecfrm5i.txt

F6: forms/F6/fecfrm6.pdf forms/F6/fecfrm6i.pdf forms/F6/fecfrm6i.txt

F7: forms/F7/fecfrm7.pdf forms/F7/fecfrm7i.pdf forms/F7/fecfrm7i.txt

F8: forms/F8/fecfrm8.pdf forms/F8/fecfrm8i.pdf forms/F8/fecfrm8i.txt

F9: forms/F9/fecfrm9.pdf forms/F9/fecfrm9i.pdf forms/F9/fecfrm9i.txt

F13: forms/F13/fecfrm13.pdf forms/F13/fecfrm13i.pdf forms/F13/fecfrm13i.txt
