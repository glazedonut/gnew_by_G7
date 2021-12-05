#!/bin/sh

. ./test-lib.sh

test_expect_success 'status shows untracked file' '
	echo foo >foo &&
	gnew status >out &&
	grep "? foo" out
'

test_expect_success 'status shows added file' '
	gnew add foo &&
	gnew status >out &&
	grep "A foo" out
'

