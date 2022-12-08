#!/bin/bash
set -e

tar -zcvf dist.tar.gz ./dist/

FTP_HOST=192.168.1.119
RELEASE_TAR_MD5=$(md5sum dist.tar.gz | cut -c1-32)
CI_COMMIT_SHORT=$(echo $CI_COMMIT_SHA | cut -c1-6)
PKG_NAME=force-gpuproxy-$1-$(echo $CI_COMMIT_SHA | cut -c1-6)
DTIME=$(date '+%Y-%m-%d_%H-%M-%S')

PUTFILE=$PKG_NAME-$CPU_PLATFORM-$DTIME-md5-$RELEASE_TAR_MD5.tar.gz

chmod -R 777 dist.tar.gz
mv dist.tar.gz $PUTFILE

ftp -v -n 192.168.1.119<<EOF
user ipfs Ipfs@123
binary
cd force-gpuproxy
lcd ./
passive
put $PUTFILE
bye
#here document
EOF
echo "commit to ftp successfully"

rm $PUTFILE

API_ADDR=http://192.168.1.141:10001/
# type 1 means Venus
type=1
programName=force-gpuproxy-$1-$CPU_PLATFORM
commitId=$CI_COMMIT_SHORT
branch=$CI_COMMIT_REF_NAME
link=ftp://$FTP_HOST/force-gpuproxy/$PUTFILE
description="branch: $CI_COMMIT_REF_NAME; commit: $CI_COMMIT_SHORT"
version="$CI_COMMIT_SHORT"

if [[ $branch == master ]]; then
	echo "push release version"
	method="pushRelease"
fi

if [[ $branch == force/test_* ]]; then
	echo "push test version"
	method="pushTest"
fi

if [[ $method ]]; then
    echo "push $programName with method $method"
	#statements
    curl -v -X PUT "$API_ADDR$method" \
        --data-urlencode "type=$type"\
        --data-urlencode "commitId=$commitId" \
        --data-urlencode "branch=$branch" \
        --data-urlencode "programName=$programName" \
        --data-urlencode "link=$link" \
        --data-urlencode "description=$description" \
        --data-urlencode "version=$version"
else
    echo "no release info"
fi
