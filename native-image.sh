#!/usr/bin/env bash

cd build/libs
rm -rf target/native
mkdir -p target/native
cd target/native
jar -xvf ../../familyphotos.jar >/dev/null 2>&1
cp -R META-INF BOOT-INF/classes
native-image -H:Name=familyphotos -cp BOOT-INF/classes:`find BOOT-INF/lib | tr '\n' ':'`
mv family-photos ../../ 
