#!/bin/bash

echo "---trace"
TRACE_START=$(date +%s)
target/release/permanent_trace analyse $1 > $1/trace.out
TRACE_END=$(date +%s)
echo "time: " $(expr $TRACE_END - $TRACE_START)
echo "time: " $(expr $TRACE_END - $TRACE_START) > $1/time.out

echo "---cig"
CIG_START=$(date +%s)
target/release/permanent_cig $1 > $1/cig.out
CIG_END=$(date +%s)
echo "time: " $(expr $CIG_END - $CIG_START)
echo "time: " $(expr $CIG_END - $CIG_START) >> $1/time.out

echo "---tester"
TESTER_START=$(date +%s)
target/release/permanent_tester $1 > $1/tester.out
TESTER_END=$(date +%s)
echo "time: " $(expr $TESTER_END - $TESTER_START)
echo "time: " $(expr $TESTER_END - $TESTER_START) >> $1/time.out
