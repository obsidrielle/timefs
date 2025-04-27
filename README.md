# TimeFS

Welcome to TimeFS! It's a file system that will automatically create and manage files' historical versions for you.


Within each directory, there is a special `.history` fold, containing historical versions of all files. Moreover, each file has its own directory to record all historical versions of it, which are named of timestamp.

```
$ ls -la /mnt/timedata/.history/
total 4
drwxr-xr-x 2 user user 4096 Apr 26 10:00 .
drwxr-xr-x 3 user user 4096 Apr 26 10:00 ..
drwxr-xr-x 2 user user 4096 Apr 26 10:00 test.txt

$ ls -la /mnt/timedata/.history/test.txt/
20230426_100015.txt
20230426_100030.txt
```

Then, TimeFS offers commands to manage these verions.

```
$ timefs versions /mnt/timedata/test.txt
Version History for /mnt/timedata/test.txt:
1. 2023-04-26 10:00:15 (12 bytes) [Initial Version]
2. 2023-04-26 10:00:30 (17 bytes) [Current Version]

$ timefs diff /mnt/timedata/test.txt@20230426_100015 /mnt/timedata/test.txt
- Hello World
+ Modified content

$ timefs restore /mnt/timedata/test.txt@20230426_100015
File restored to version from 2023-04-26 10:00:15

$ timefs cleanup --keep-days=30 --max-per-day=1 /mnt/timedata/
Cleaned up 15 old versions, freed 1.2MB of space
```

Options:

```
$ timefs mount /data /mnt/timedata \
    --auto-version=true \
    --min-interval=5m \
    --max-versions=100 \
    --exclude="*.tmp,*.log" \
    --storage-limit=10G$ timefs mount /data /mnt/timedata \
    --auto-version=true \
    --min-interval=5m \
    --max-versions=100 \
    --exclude="*.tmp,*.log" \
    --storage-limit=10G
```