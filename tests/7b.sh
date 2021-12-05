#!/bin/sh

. ./test-lib.sh

test_expect_success 'push does not succeed if remote and local differ' '
	mkdir remote &&
	cd remote &&
	gnew init > /dev/null &&
	test_commit init file.txt &&
	cp -R . ../local &&
	test_commit foo foo.txt &&
	cd ../local &&
	test_commit bar bar.txt &&
	gnew push ../remote 2>&1 | grep fatal
'

test_expect_success 'pull merges correctly' '
	gnew pull ../remote &&
	cat .gnew/tracklist | grep foo.txt
'
