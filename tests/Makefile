T = $(sort $(wildcard [1-7][a-z].sh))

test:
	@for test in $(T); do echo "*** $$test ***"; ./$$test; done
