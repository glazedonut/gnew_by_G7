#!/bin/sh

. ./test-lib.sh

test_expect_success 'heads shows current branch' '
	gnew heads >out &&
	grep "* main" out
'
