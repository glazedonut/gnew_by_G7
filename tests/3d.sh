#!/bin/sh

. ./test-lib.sh

test_expect_success 'log shows commit author and message' '
	gnew commit "test" 
	gnew log >out &&
	grep G7 out &&
	grep "test" out
'
