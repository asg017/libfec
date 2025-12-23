
.PHONY: test-files

.test-files/1885517.fec:
	echo cargo run download $(basename $@)

.test-files/1913493.fec:
	echo cargo run download $(basename $@)

test-files: .test-files/1885517.fec .test-files/1913493.fec
