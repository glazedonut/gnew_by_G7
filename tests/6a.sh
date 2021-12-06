#!/bin/sh

. ./test-lib.sh

test_expect_success 'clone clones a remote repository' '
	mkdir local &&
	mkdir remote &&
	cd remote &&
	gnew init &&
	cd ../local &&
	gnew clone ../remote &&
	ls -la remote | grep .gnew
'
