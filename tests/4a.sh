#!/bin/sh

. ./test-lib.sh

test_expect_success 'remove removes files from the tracklist' '
	test_commit "remove me" file &&
	gnew remove file &&
	gnew status >out &&
	grep "R file" out
'
