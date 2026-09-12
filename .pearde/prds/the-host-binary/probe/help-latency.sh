#!/bin/sh
n=200
t0=$(perl -MTime::HiRes=time -e 'print time')
i=0; while [ $i -lt $n ]; do /usr/bin/true; i=$((i+1)); done
t1=$(perl -MTime::HiRes=time -e 'print time')
i=0; while [ $i -lt $n ]; do ./target/debug/zirkle --help >/dev/null; i=$((i+1)); done
t2=$(perl -MTime::HiRes=time -e 'print time')
perl -e "printf(\"true   %.3f ms/run\nzirkle %.3f ms/run\nnet    %.3f ms/run\n\", ($t1-$t0)*1000/$n, ($t2-$t1)*1000/$n, (($t2-$t1)-($t1-$t0))*1000/$n)"
