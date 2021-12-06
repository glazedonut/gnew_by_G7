
#!/bin/sh

. ./test-lib.sh

test_expect_success 'clone does not replace repository if it already exists' '
	mkdir local &&
	mkdir remote &&
	cd remote &&
	gnew init &&
	cd ../local &&
	gnew clone ../remote &&
	gnew clone ../remote 2>&1 | grep fatal
'
