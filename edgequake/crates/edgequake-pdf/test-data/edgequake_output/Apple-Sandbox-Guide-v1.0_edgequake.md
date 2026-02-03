## Page 1

Apple

's Sandbox Guide

v1.0

13

2011


---

## Page 2

v1.0

**Table of Contents**

- Introduction

- What are we talking about?

**3  - How can it be used or implemented?**

## 4  Anatomy of a custom profile

## 5  Commands Reference

5.1  - Actions  ................................

5.2  - Operations  ................................

5.3  - Filters  ................................

5.4  - Modifiers................................

### 5.5  Other keywords

**6  - Special hardcoded cases**

**7  - A sample sandbox profile**

**8  - Where to find more information**

### Appendix A

### Appendix B

**.....**

**.............. 3**

**............. 5**

31

35

...............  36

**................. 37**

**................ 37**

**..... 43**

**.......... 43**

**........... 44**


---

## Page 3

v1.0

- Introduction

Apple's sandbox technology was introduced in

Leopard version of Mac OS X

and was based on TrustedBSD MAC framework

. 

A few years have passed and documentation is still 

paper and presentation 

at Blackhat DC 2011, reversing

of this feature. T

he other available public references are Apple's own sandbox profiles 

/usr/share/sandbox) 

and some attempts by other users to create new profiles

for some things I found

ad ditional documentation. These references are available at

notably forgot about The Mac Hacker's Handbook!

This document tries to

close this gap

by trying to document

this technology.

It is a work in prog

ress and based on Snow Leopard v10.6.8

this technology as private and subject to changes

I have tried to provide examples for all operations so it is easier to understand their impact and 

mode of operation.

There is a very real pos

sibility of mistakes and wrong assumptions so a

are more than welcome!

You can contact me at 

The latest version is always available at 

http://reverse.put

Enjoy,

- What are we talking about?

Using the definition from Apple's we

bsite:

" Sandboxing protects the system by limiting the kinds of operations an application 

can perform, such as opening documents or accessing the network. Sandboxing 

makes it more difficult for a security threat to take advantage of an issue in a 

specific application to affect the greater system."

The implementation found in Mac OS X can limit the following type of operations:

- File: read, write, with many different

granular 

- IPC: Posix and SysV
- Mach
- Network: inbound, outbound
- Process: execution, fork
- Signals
- Sysctl
- System

It has a rich set of operations that can help to improve the security of applications and mitigate 

. It was called Seatbelt 

scarce. Dionysus Blazakis

published a great 

the userland and kernel 

implementation

(located at 

. While searching 

chapter 8. I 

the operations and options

available in 

. Apple still considers 

.

ll contributions and fixes 

.as.

reverser@put.as

potential attacks, especially on network

                - enabled applications such as web browsers, Flash or

applications that process 

potentially untrusted input such as pd

https://media.blackhat.com/bh

-dc -11/Blazakis/BlackHat_DC_2011_Blazakis_Apple_Sandbox

https://github.com/s7ephen/OSX

or tweet @osxreverser.

f, word/excel/powerpoint 

                          - wp.pdf

operations


---

## Page 4

v1.0

documents, etc.

Malware analysis and reverse engineering processes can also benefit from this 

technology.

**3  - How can it be used or implemented?**

There are two 

alternatives to use this feature

(one is just a frontend for the other)

The first is to execute an application within a sandbox, using the command "sandbox

the best alternative for applying a sandbox to software you don't have source code.

The other is to implement

the sandbox feature inside your code

function "sandbox_init"

will place the process into a sandbox

below ( they are also available to sandbox

                - exec, although with a different name

These profiles are:

- kSBXProfileNoInternet : TCP/IP netw

orking is prohibited.

- kSBXProfileNoNetwork : All sockets
              - based networking is prohibited.
- kSBXProfileNoWrite : File system writes are prohibited.
- kSBXProfileNoWriteExceptTemporary : File system writes are restricted to the temporary folder

/var/tmp and the 

folder specified by theconfstr(3) configuration variable 

_CS_DARWIN_USER_TEMP_DIR.

- kSBXProfilePureComputation : All operating system services are prohibited.

Check the sandbox_init manpage for more information. 

OS X Lion introduces Application Sandboxing

, a different way of applying sandboxing but with the 

same underlying technology.

Now let's focus

on sandbox exec the best alternative for most users.

The sandbox exec supports the pre

              - defined profiles but also custom profiles. Custom profiles are

writte n in SBPL - Sandbox Profile Language (a "

Scheme embedded domain specific language" 

using Dion's definition). 

E xamples can be found 

sandbox some system daemons.

The next chapters describe the different operations, 

modifiers available to write custom profiles.

.

                                  - exec". This is

or someo ne else's code.

using one of the pre

                                - defined profiles

at "/usr/share/sandbox". These are used to 

filters and 

http://developer.apple.com/library/mac/#documentation/Security/Conceptual/CodeSigningGuide/Introduction/Introduction.html#//a

ref/doc/uid/TP40005929

-CH1 -SW1

pple_

The syntax for 

sandbox exec command is:

command [arguments ...]

The - f switch should be used for loading cus

tom profiles. Either you can use the absolute path

the profile or just the name of the profile

, as long it is located at

/Library/Sandbox/Profiles

/System/Library/Sandbox/Profiles

/usr/share/sandbox

Using f ull path example:

Using profile n

ame example:

one of the these folders:


---

## Page 5

v1.0

where bsd.sb is located at /usr/share/sandbox

You can also use custom profiles writing from the input, using the 

The example from Dion'

s paper:

> (version 1)

> (allow default) 

> (regex #"^/private/tmp/dump

\.c$")) 

> ' /bin/sh

The - n switch is used to load

one of the pre

                - defined profiles.

names are differ

ent from sandbox_init. Use the following table as reference.

Sandbox_init

kSBXProfileNoInternet

kSBXProfileNoNetwork

kSBXProfileNoWriteExceptTemporary

kSBXProfileNoWrite

kSBXProfilePureComputation

Example:

PING www.l.google.com (209.85.148.106): 56 data bytes

ping: sendto: Operation not permitted

Logging, by default, is written to "/var/log/system.log". 

"tail - f" to that file while developing your custom profiles.

                                  - p switch.

As previously pointed out, the profiles 

Sandbox exec

no internet

no network

no write

pure computation

It is very useful (well, essential!) to do a 

ted. Then it can be simplified using the 

appear!). 

operations can have filters to improve granularity, 

while 

others are binary 

only (allow or deny

One available feature is automatic generation of rules using the trace feature. A trace file with 

automatic rules for denied operations will be crea

"sandbox simplify" command and integrated in your scripts. Please check point 5.5.2 for more

details (the operation isn't as automatic as it might 

## 4  Anatomy of a

**custom  profile**

A profile is composed of 

actions on operations, modifiers, filters, options and (optionally) 

comments. To simplify things I will call everything

commands except for comments.

The core of custom profiles are operations, which are what you want to control and limit access to. 

Exampl es of operations are read files, write files, access a network port, send signals, etc.

Most 


---

## Page 6

v1.0

The first thing to be configured is the version of 

should be common to all scripts. 

Additionally you can configure the logging option, with the "debug" 

available, " all ", which should log all operations (allowed or not), and 

operations. The option "all" doesn't seem to work

level?) but the "deny" option is very useful at the custom profile writing and debugging stage.

Other profiles can be included using the 

" import

rules to be shared among daemons, which is what bsd.sb is for BSD daemons

The default action can be configured either to deny or to allow. This will depend on the type of 

profile you are interested to achieve.

Comments should start with semi

colon (;) and are valid until the end of the line

## 5  Commands Reference

All commands are enclosed into parenthesis. In each example, the "$" symbol means command 

execution at the shell.

**5.1  - Actions**

There are two available

actions, allow or deny.

Actions apply only to the operations defined below.

Syntax:

( action operation [filter modifiers])

Example:

- (deny default)

All operations will be denied unless explicitly allowed (default is an operation). This is a whitelist 

mode.

- (allow default)

All operations will be allowed unless explicitly denied. In this case we have a blacklist mode.

the SBPL. For now there's only version 1 so this 

command. Two options are 

" deny ", whic h logs only denied 

(not implemented? requires a different log 

" command, for example a profile with common 

.

.

Operations can have filters and modifiers. Modifiers 

while filters don't.

apply to all operations

(e xcept the mach ones

**5.2  - Operations**

As previously described, the sandbox supports different type of operations. 

have global and granular mode

s. Global means that the whole category of operation can be 

configured. For example, the "file*" operation will control all type of file related operations. But we 

can also be more granular and allow file reads and deny file writes (and even be a little mo

specific in these two operations).

The following table shows the global operations, including the ones without granular modes.

Default

File*

Ipc*

Signal Sysctl* System*

Almost all operations 

Mach*

Network*

Process*


---

## Page 7

v1.0

I have tried to find all kernel functions where operations apply (for example, an operation such as 

file write xattr is implemented in a few kernel functions). This is described in the ite

when available.

All the available operations are now described.

### Default

Syntax:

(action default [ modifier])

Actions:

allow deny

Filters:

Description:

As the name implies, this is the default action if no

where this operation is configured, either at the beginning or the end of the profile. The engine will 

only hit the default operation if no explicit match can be found. Searching for operations will stop 

when the first explicit match is hit. This means that a deny action followed by an allow action to 

the same operation and target will never trigger the allow action, it will always be denied.

All operations that Sandbox module supports/implements.

Example s:

(allow default)

To create a blacklist

profile.

(deny default)

To create a whitelist profile.

(deny default (with no

        - log))

To create a whitelist profile without logging.

### File*

Syntax:

(action file* [filter] [modifier])

Actions:

allow deny

Filters:

path file mode

m Applies to, 

other operation matches. It doesn't matter 

(deny file*)

T his will deny all file 

related operations to any file.

(deny file* (literal "/mach_kernel"))

Description:

This operation will control file related operations such as reads, writes, extended attributes, etc.

All file operations described in detail below.


---

## Page 8

v1.0

This will deny all file related operations that have /mach_kernel as target.

### File chroot

(action file chroot [filter] [modifier])

Description:

Control whether the target should be allowed or not to chroot() into the specified directory.

chroot, link

Example(s):

(deny file chroot (literal "/"))

chroot: /: Operation not permitted

Log output:

Sep 2 18:45:02 

macbox sandboxd[40841]: chroot(40840) deny file

### File ioctl

(action file ioctl [filter] [modifier])

Description:

Determine whether the 

target can perform the ioctl operation

Warning: Since ioctl data is opaque from the standpoint of the MAC

can affect many aspects of system

operation, policies 

implementing

access control checks.

vn_ioctl, link

Example(s):

(allow file ioctl (literal "/dev/dtracehelper"))

### File read*

(action file read* [filter] [modifier])

                          - chroot /

.

framework, and since ioctls 

must exercise extreme care when 


---

## Page 9

v1.0

Description:

Controls all available read operations described below.

e xchangedata, link

Example(s):

(deny file read* (literal "/mach_kernel"))

cat: /mach_kernel: Operation not permitted

Sep 2 00:13:12 

macbox sandboxd[24486]: cat(24485) deny file

ls: /mach_kernel: Operation not permitted

Sep 2 00:13:46 

macbox sandboxd[24498]: ls(24504) deny file

xattr: No such file: /mach_kernel

macbox sandboxd[24498]: Python(24497) deny file

macbox sandboxd[24498]: Python(24497) deny 

(action file read data [filter] [modifier])

allow deny

| access1 Applies to: Give or refuse read access to  Description:   | ,  getvolattrlist |  |  |  |  |  |  |  |  |
|---|---|---|---|---|---|---|---|---|---|
| Modifiers: Filters:   Actions: Syntax:   |         |  |  |  |  |  |  |  |  |
| File | read | /mach_kernel | Sep | 2 | 00:13:38 | Sep | 2 | 00:13:38 |  |

path file mode

send signal no log

cat: /mach_kernel: Operation not permitted

*Sep 2 00:18:59* 

*macbox sandboxd[24653]: cat(24652) deny file*

target file

, link, __mac_mount

(deny file read data (literal "/mach_kernel"))

/mach_kernel

contents.

, vn_open_auth

, union_dircheck


---

## Page 10

v1.0

### File read metadata

Syntax:

(action file read metadata [filter] [modifier])

Acti ons: allow deny

Filters:

path file mode

Description:

Control read access to the files

            - system metadata. For example "ls" will not work against the target

(if action is deny) while a "cat" will (because it is accessing the 

getattrlist_internal

, link, namei, readlink, CheckAccess

(deny file read metadata (literal "/mach_kernel"))

$ cat /mach_kernel

????uZ$

cat: /mach_kernel: 

Operation not permitted

Log output:

Sep 2 00:24:11 

macbox sandboxd[24809]: ls(24808) deny file

### File read xattr

Syntax:

(action file read xattr [filter] [modifier])

Actions:

allow deny

Filters:

path file mode xattr

contents, not the metadata).

, vn_stat

xattr: [Errno 1] Operation not permitted: '/mach_kernel' Description: This operation will control read access to file

's extended attributes.

vn_getxattr, link, vn_listxattr

(deny file read xattr (literal "/mach_kernel")

Result without sandbox:

$ xattr /mach_kernel

com.apple.FinderInfo

Result with sandbox:


---

## Page 11

v1.0

### File revoke

(action file revoke [filter] [modifier])

Description:

Controls access to revoke (v

oid all references to file by ripping underlying filesystem

vnode).

r evoke, link

### File write*

(action file write* [filter] [modifier])

Description:

Controls all available write operations described below.

unp_bind, mknod, mkfifo1, symlink, mkdir1, 

setattrlist_internal

, unlink1, rmdir

(deny file write* (literal "/test"))

touch: /test: Operation not permitted

Log output:

Sep 2 21:05:46 

macbox sandboxd[45341]: touch(45340) deny file

### File write data

(action file write data [filter] [modifier])

away from 

vn_open_auth

, exchangedata

, link, rename,

                          - write* /test


---

## Page 12

v1.0

Description:

Give or refuse write access to the contents of the target file.

Warning: this doesn't seem to work

as expected if action is deny

- content can't be read
        - but for some reason this one doesn't deny write contents to the target

file (only file write* works).

For example this works (data is 

written):

(allow file write data

(literal "/private/tmp/test3")

(deny file write* (literal "/private/tmp/test3"))

While this doesn't work (data is written when it shouldn't):

(deny file write data

(literal "/private/tmp/test3")

(allow file write* (literal "/private/tmp/test3"))

Or this also doesn't work (data is written when it shouldn't):

(allow default)

(deny file write data (literal "/private/tmp/test3"))

g etvolattrlist, access1, link, __mac_mount,vn_open_auth, union_dir

truncate, ftruncate

(deny file write data (literal "/private/tmp/test3"))

### File write flags

Syntax:

(action file write flags [filter] [modifier])

Actions:

allow deny

Filters:

path file mode

! File read data works as expected

check, fcntl_nocancel

chflags: /tmp/test: Operation not 

permitted

Description:

Control access to file flags (check manpage for chflags).

chflags1, link

(deny file write flags (literal "/private/tmp/test"))


---

## Page 13

v1.0

Log output:

Sep 2 19:29:59 

macbox sandboxd[42198]: chflags(42197) deny file

/private/tmp/test

### File write mode

(action file write mode [filter] [modifier])

Description:

Control access to file modes.

chmod2, link

(deny file write mode (literal "/private/tmp/test"))

chmod: Unable to change file mode on /tmp/test: Operation not permitted

Log output: 

Sep 2 19:54:35 

macbox sandboxd[43051]: chmod(43050) deny file

/private/tmp/test

### File write mount

(action file write mount [filter] [modifier])

Description:

Access control check for mounting a file system

.

__mac_mount

, prepare_coveredvp

, mount_begin_update

N/A (tried different combinations and mount still works!)

### File write owner

(action file write owner [filter] [modifier])


---

## Page 14

v1.0

Control access to file ownership changes.

| , link |
|---|
|   |

chown1, fchown

(deny file write owner (literal "/private/tmp/test"))

chown: /tmp/test: Operation not permitted

Log output:

Sep 2 20:05:48 

macbox sandboxd[43419]: chown(43418) deny file

/private/tmp/test

### File write setugid

Syntax:

(action file write setugid [filter] [mo

difier])

Actions:

allow deny

Filters:

path file mode

Access control check for setting

file mode. It (seems to) apply only to suid and sgid bits.

c hmod2, link

(deny file write setugid (regex "^/private/tmp/.*"))

Log output:

Sep 12 22:46:57 

macbox sandboxd[80230]: chmod(80229) deny file

/private/tmp/test

--- S ------ 1 root staff 9 Sep 12 22:46 /Users/

reverser/test

### File write times

Syntax:

(action file write times [filter] [modifier])

Actions:

allow deny

Filters:

path file mode

Control s et the access and modification times of a file. 


---

## Page 15

v1.0

setutimes, link

### File write unmount

Syntax:

(action file write unmount [filter] [modifier])

Actions:

allow deny

Filters:

path file mode

Description:

Access control check for unmounting a filesystem

u nmounts, link

(deny file write unmount (literal "/Volumes/Mac OS X Install ESD"))

umount: unmount(/Volumes/Mac OS X Install ESD): Operation not permitted

Log output:

Sep 2 20:21:19 

macbox sandboxd[43908]: umount(43911) deny file

### File write xattr

Syntax:

(action file write xattr [filter] [modifier])

Actions:

allow deny

Filters:

path file mode xattr

.

xattr: [Errno 1] Operation not permitted: '/test' Description: This operation will control write access to the file extended attributes.

vn_removexattr

, link, vn_setxattr

(deny file write xattr (literal "/test"))

test: 123


---

## Page 16

v1.0

Log output:

Sep 2 00:38:13 

### Ipc*

| send n/a   allow deny (action ipc* [modifier]) | - signal no   | - log |     |  |
|---|---|---|---|---|
| macbox |   |   sandboxd[25217]: Python(25216) deny file | - write | - write - xattr /test |

Modifiers:

This operation will IPC related operations described below.

All IPC operations described below.

(deny ipc*)

### Ipc posix*

(action ipc posix* [modifier])

allow deny

This operation will IPC POSIX related operations described below.

All IPC - Posix operations described below.

### Ipc posix sem

(action ipc posix sem [modifier])

allow deny

sem_open, sem_post, sem_unlink, sem_wait_nocancel

, sem_trywait

Controls access to POSIX semaphores

functions (create, open, post, unlink, wait).


---

## Page 17

v1.0

(allow ipc posix sem)

### Ipc posix shm

| -   | - signal no   - posix | - log   - posix - |
|---|---|---|
|   |  |  |

Modifiers:

Description:

Controls access to POSIX shared memory region 

unlink).

shm_open, pshm_mmap

, pshm_stat, pshm_truncate

(allow ipc posix shm)

### Ipc sysv*

(action ipc sysv* [modifier])

allow deny

Description:

This operation will 

control all IPC SysV related operations described below.

All IPC SysV operations described 

below.

(allow ipc sysv*)

### Ipc sysv msg

(action ipc sysv msg [modifier])

allow deny

functions (create, mmap, open, stat, truncate, 

, shm_unlink

msqrcv, msqsnd).

msgsnd_nocancel

, msgrcv_nocancel

, msgctl, msgget

Definition:

Controls access to System V messages

functions

(enqueue, msgrcv, msgrmid, msqctl, msqget, 


---

## Page 18

v1.0

(allow ipc

### Ipc sysv sem

| -     | - signal no   - sysv | - log   - sysv - sem [modifier]) |
|---|---|---|
| - sysv - | - msg)   |   |

Modifiers:

Controls access to System V semaphores 

functions 

Semctl, semget, semop

(allow ipc sysv sem)

### Ipc sysv shm

(action ipc sysv shm [modifier])

allow deny

Controls access to mapping System V shared memory

Shmat, shmctl, shmdt, shmget_existing

(allow ipc sysv shm)

### Mach*

(action mach* [modifier])

allow deny

(semctl, semget, semop).

functions (shmat, shmctl, shmdt, shmget).

All Mach operations described below.

Controls access to all Mach related functions described below.


---

## Page 19

v1.0

(deny mach*)

### Mach bootstrap

| allow deny   (action mach |
|---|
|   |

Used only to apply sandbox? To create/access new mach ports

### Mach lookup

(action mach lookup [modifier])

allow deny

mach

Mach IPC communications/m

essages.

Most applications require access to some basic Mach services (bsd.sb configures 

(allow mach lookup

(global name "com.apple.system.logger")

(global name regex #"^com.apple.DeviceLink.AppleMobileBackup*")

### Mach priv*

(action mach priv* [modifier])

allow deny

the basic ones).

Control access to all the mach

          - priv operations defined below.


---

## Page 20

v1.0

set_security_token

Example (s)

| - priv*)   | port   |
|---|---|
|  | , task_for_pid |

(allow mach

### Mach priv

(action mach priv host port [modifier])

Access control check for 

retrieving a process's host port.

set_security_token

### Mach priv task port

(action mach priv task -port [modifier])

Access control check for getting a process's 

task port, 

apply, such as task_for_pid only available to root or group procmod.

task_for_pid

### Mach task name

(action mach task -name [modifier])

task_for_pid()

. Standard restrictions still 


---

## Page 21

v1.0

Access control check for getting a process's 

task name, 

restrictions still apply, such as task_for_pid only available to root or group procmod.

task_name_for_pid

Example (s)

(deny mach

| - task   | deny - name)   |
|---|---|
|  |  |

### Network*

Syntax:

Actions:

allow deny

Filters:

network path file

Definition:

Controls all available network operations described below.

be localhost or * (check network filter for more information).

soo_read, soreceive, recvit, listen, b in d, unp_bind

unp_connect

(deny network* (remote ip "*:80"))

Log output:

Sep 2 21:12:00 

macbox sandboxd[45542]: nc(45540) deny network

### Network inbound

Syntax:

(action network

          - inbound [filter] [modifier])

Actions:

allow deny

Filters:

network path file

task_name_for_pid()

. Standard 

It has no support for IP filtering, it must 

, connect_nocancel

, sendit, soo_write, 

                            - outbound 74.125.39.99:80

from the socket's file descriptor.

soo_read, soreceive, recvit, listen Definition: Control s network inbound operations.

It has no support for IP filtering, it must be localhost or * 

(check network filter for more information).

" A socket has a queue for receiving incoming data. When a packet arrives

eventually gets deposited into

this queue, which the

on the wire, it 

owner of the socket drains when they read 


---

## Page 22

v1.0

(allow network

      - inbound (local ip4 "*:22))

### Network bind

Syntax:

(action network

odifier])

Actions:

allow deny

Filters:

network path file

Definition:

Control access to socket bind().

It has no support for IP filtering, it must be localhost or * (check 

network filter for more information).

nc: Operation not permitted

Log output:

Sep 2 21:08:41 

macbox sandboxd[45438]: nc(45437) deny network

### Network outbound

Syntax:

(action network

          - outbound [filter] [modifier])

Actions:

allow deny

Filters:

network path file

                            - bind 0.0.0.0:7890

Sep 2 22:29:03 

macbox sandboxd[47760]: nc(47758) deny ne

74.125.39.106:80

(allow network

      - outbound (remote unix - socket (path

twork outbound

                      - literal "/private/var/run/syslog")))

Definition:

Controls access to send data to the socket.

It has no support for IP filtering, it must be localhost or 

(check network filter for more information).

connect_nocancel

, sendit, soo_write, unp_connect

Example ( s):

      - outbound)

This will deny any packets going out from the 

target application.

Log output:


---

## Page 23

v1.0

Allow access to the syslog unix socket.

### Process*

Syntax:

(action process* [modifier])

Actions:

allow deny

Filters:

Definition:

Controls all available process operations described 

are available here but are on process

              - exec.

link, exec_check_permissions

, getvolattrlist, access1

(deny process*)

sandbox exec: ls: Operation not

permitted

Log output:

Sep 2 22:36:09 

macbox sandboxd[47975]: sandbox

### Process exec

Syntax:

(action process

          - exec [filter] [modifier])

Actions:

allow deny

Filters:

path file mode

Modifiers: send signal no log no sandbox

Definition:

Control process execution.

l ink, exec_check_permissions

, getvolattrlist, access1

(deny process

      - exec (literal "/bin/ls"))

sandbox exec: /bin/ls: Operation not permitted

sandbox exec: ls: Operation not permitted

Log output:

Sep 2 01:16:57 

macbox sandboxd[26360]: sandbox

below. One important detail is that no filters 

, fork1

                    - exec(47980) deny process

Sep 2 01:17:00 

macbox sandboxd[26360]: sandbox

                    - exec(26363) deny process

                    - exec(26359) deny process


---

## Page 24

v1.0

### Process fork

(action process

          - fork [modifier])

Actions:

allow deny

Filters:

Definition:

Control access to fork and vfork.

fork1

(deny process

      - fork)

$ ./forktest 

child!

parent!

parent!

Log output:

Sep 2 01:23:52 

macbox sandboxd[26677]: forktest(26676) deny process

### Signal

(action signal [filter] [modifier])

Actions:

allow deny

Filters:

signal

### Sysctl*

(action sysctl* [modifier])

Definition:

Control if program can send signals to itself, processes in the same group or all other processes.

cansignal

(deny signal (target others))

The sandboxed process will not be able to send signals to other processes.

kill: 10229: Operation not permitted

Log output:

Sep 2 10:45:01 

macbox sandboxd[ 31416]: kill(31418) deny signal


---

## Page 25

v1.0

Definition:

Control all access to sysctl() and its variants, 

sysctlbyname

sysctl, sysctlbyname

, sysctlnametomib

(deny sysctl*)

Log output:

Sep 2 01:33:50 

macbox sandboxd[26952]: sysctl(26960) deny sysctl

second level name bpf_bufsize in debug.bpf_bufsize is invalid

This happens because sysctl

          - read is also denied so it can't read the name.

### Sysctl read

Syntax:

(action sysctl read [modifier])

Definition:

Control read access to sysctl() and its variants, 

sysctlbyname

s ysctl, sysctlbyname

, sysctlnametomib

(deny sysctl read)

Log output:

Sep 2 01:40:01 

macbox sandboxd[27171]: sysctl(27170) deny sysctl

second level name bpf_bufsize 

in debug.bpf_bufsize i

### Sysctl write

Syntax:

(action sysctl write [modifier])

and sysctlnametomib

.

and sysctlnametomib

.

s invalid


---

## Page 26

v1.0

Definition:

Control write access to sysctl() and its variants, 

sysctlbyname

Note: there seems to be a bug in this implementation (Snow Leopard at least)

sysctl write) requires a (allow sysctl

            - read), even if we have a (allow default).

Test command:

Test profile:

(version 1)

(debug all)

(allow default)

(deny sysctl write)

But it works if written this way:

(version 1)

(debug all)

(allow default)

(deny sysctl write)

(allow sysctl read)

S ysctl

Example:

### System*

Syntax:

(action system* [modifier])

Actions:

allow deny

Filters:

and sysctlnametomib

.

, where a (deny 

2200

Sep 2 22:49:30 

macbox sandboxd[48428]: d

ate(48435) deny system

Definition:

Controls all available system operations described below.

acct, setaudit, setauid, audit, auditon, auditctl, 

fsctl

socket, macx_swapoff,

macx_swapon, fcntl

(deny system*)

date: settimeofday (timeval): Operation not permitted

Log output:

, setlcid, nfssvc, reboot, s ettimeofday, adjtime, 


---

## Page 27

v1.0

### System acct

| send - n/a   allow deny (action system | - signal no - log   - acct [modifier]) |
|---|---|
|  |  |

Modifiers:

Determine whether the 

target should be allowed to enable accounting,

lab el of the accounting log file. 

See acct(5) for more information.

(allow system

      - acct)

### System audit

(action system

          - audit [modifier])

allow deny

Determine whether the 

target can submit an audit record for inclusion in the audit log via the 

audit() system call.

setaudit, setauid, audit, auditon, auditctl

(allow system

      - audit)

### System fsctl

(action process* [modifier])

allow deny

based on its label and the 

Control access to fsctl().


---

## Page 28

v1.0

Warning: The fsctl() system call is directly analogous to ioctl(); since

from the standpoint of the MAC framework and since these operations can affect ma

system operation,

policies must exercise extreme care when implementing access control checks.

fsctl

(deny system fsctl)

### System lcid

          - lcid [modifier])

Actions:

allow deny

Filters:

Description:

Determine whether the 

target can relabel itself to the supplied new label (newlabel). This access 

control check

is called when the mac_set_lctx/lcid syste

application will supply

a new value, the value will be internalized and provided in newlabel.

setlcid

(allow system

      - lcid)

### System mac label

Actions:

allow deny

Filters:

the associated data is opaque 

ny aspects of 

m call is invoked. A user space

### System nfssvc

          - nfssvc [modifier])

Description:

Determine whether the 

target can perform the mac_set_fd operation. The mac_set_fd operation 

is used to associate a MAC label with a file.

(deny system mac label)


---

## Page 29

v1.0

Determine whether the 

target should be allowed to call nfssrv(2).

nfssvc

(allow system

      - nfssvc)

### System reboot

Syntax:

(action system

          - reboot [modifier])

Controls if target can reboot system.

Note: doesn't seem to work!

reboot

(deny system reboot)

### System set time

Syntax:

(action system set time [modifier])

2200

date: settimeofday (timeval): Operation not permitted

Log output:

Controls access to the system clock.

s ettimeofday, adjtime

(deny system set time)


---

## Page 30

v1.0

Sep 2 22:49:30 

macbox sandboxd[48428]: date(48435) deny system

### System socket

|   send n/a   allow deny (action system | - signal no - log   - socket [modifier]) |
|---|---|
|   |  |

Modifiers:

Control access to create

sockets.

socket

(deny system socket)

### System swap

(action system

          - swap [modifier])

allow deny

Access control check 

for swap devices

(swapon/swapoff).

macx_swapoff,

macx_swapon

(allow system

      - swap)

### System write bootstrap

(action system

allow deny

fcntl


---

## Page 31

v1.0

### Job creation

Syntax:

(action job creation [filter] [modifier])

Actions:

allow deny

Filters:

path

Not implemented ???

### Mach per user lookup

Syntax:

(action mach per user -lookup [modifier])

Actions:

allow deny

Filters:

**5.3  - Filters**

Filters can be applied to 

the operations that support them, allowing better control 

The filters c an be path names, file names, IP

addresses, extended attributes, file modes. Regular 

expressions are supported

where described

.

The following table resumes the existing fil

ters:

path

network

file mode

and granularity. 

Match filenames or paths.

Three different modes are supported

, regular expressions, literal, and subpath.

Anything included in square braces "[]" is optional. 

#### 5.3.1  Path

xattr

mach

signal


---

## Page 32

v1.0

Symlinks are resolved

so a path filter (literal or regex matching the beginning) to "/tmp/testfile" 

will fail because "/tmp" is a symbolic link to "/private/tmp". In this case the correct filter should 

be "/private/tmp/testfile".

Regular Expressions

(regex EXPRESSION)

(allow file read* (regex #"^/usr/lib/*"))

This will allow file reading access to all files available under /usr/lib/.

Multiple regular expressions are supported, so the operation can apply to multiple paths and/or 

files.

(allow file read*

(regex

#"^/usr/lib/*"

#"^/dev/*"

#"^/System/Library/Frameworks/*"

Literal

(literal PATH)

(deny file read* (literal "/dev"))

This will deny all file read access to /dev only, but everything else inside /dev is

this operation.

ls: /dev: Operation not permitted

/dev/dtrace

3. Subpath

(subpath PATH)

Note: the PATH never ends with a slash (/).

(deny file read* (subpath "/dev"))

In this case, everything under /dev will be denied read access (including /dev itself).

#### 5.3.2  Network

n't protected by 

1.

2.

Description:

Filter by network protocol and source or destination.


---

## Page 33

v1.0

Syntax:

(local ip|ip4|ip6|tcp|tcp4|tcp6|udp|udp4|udp6 

[ "IP:PORT"

(remote ip|ip4|ip6|tcp|tcp4|tcp6|udp|udp4|udp6 

[ "IP:PORT"

(remote unix|unix

      - socket [path - literal PATH])

The default "IP:PO

RT" is "*:*". The only valid input for IP is localhost or *, meaning that you can 

only filter by port.

The a liases "from",

"to", and "unix socket" ca n be used instead of "local",

The ICMP protocol is included in the IP and UDP options.

Note:

In this case, PATH must be "path

            - literal" instead of "regex", "literal", or "subpath".

(deny network* (remote

ip "*:*"))

Deny IP access to any remote host

.

PING www.l.google.com (74.125.39.147): 56 data bytes

ping: sendto: Operation not permitted

Sep 2 11:00:17 

macbox sandboxd[31870]: ping(31869) deny network

74.125.39.147:0

(deny network* (remote tcp "*:*"))

Deny TCP access to any remote host.

Trying 74.125.39.147...

telnet: connect to address 74.125.39.147: Op

eration not permitted

Sep 2 11:02:20 

macbox sandboxd[31937]: telnet(31935) deny network

74.125.39.147:80

(deny network* (local tcp "*:*"))

Deny TCP access to localhost ports.

Trying 127.0.0.1...

telnet: connect

to address 127.0.0.1: Connection refused

Sep 2 11:04:49 

macbox sandboxd[32011]: telnet(32010) deny network

"remote", and "unix".

                            - outbound
                            - outbound

(allow network* (remote unix

            - socket (path - literal "/private/var/run/syslog")))
                            - outbound 127.0.0.1:22


---

## Page 34

v1.0

#### 5.3.3  File mode

Match file mode 

bits.

where FILEMODE is composed of 4 bits.

Note: The match will be successful is each bit is equal or higher, meaning by this that a #o0644 

will be successfully matched by a file with a mode of 0644, 0744, 

(file mode #o0644)

Filter will match if target has permissions of 0644 (

#### 5.3.4  Xattr

Match the extended attribute name, not content.

(xattr REGEX)

(deny file write xattr (xattr "test_xattr"))

Deny writing the extended attribute named "test_xattr" to any file.

test_xattr: aaaa

xattr: [Errno 1] Operation not permit

ted: '/tmp/xattr'

Log output:

Sep 2 11:48:02 

macbox sandboxd[33295]: Python(33294) deny file

/private/tmp/xattr

#### 5.3.5  Mach

These are needed for things like getpwnam, hostname changes, & keychain

the difference between global

          - name and local
                  - name.

(global name LITERAL)

(global name regex REGEX)

(local name LITERAL)

0657, etc.

(local name regex REGEX)

(allow mach lookup (global name "com.apple.bsd.dirhelper"))

. I don't know what's 


---

## Page 35

v1.0

(allow mach lookup (global

#### 5.3.6  Signal

Description:

Filter the targets of the signals.

Syntax:

(target self|pgrp|others)

w her e,

self: sandboxed process itself

pgrp: group processes ?

others: all processes 

(deny signa l (target others))

The sandboxed process will not be able to send signals to other processes.

kill: 10229: Operation not permitted

Log output:

Sep 2 10:45:01 

macbox sandboxd[31416]: kill(31418) deny signal

**5.4  - Modifiers**

There are three available modifiers, although one just applies to a single operation. The modifiers 

are send signal, no log, and no sandbox. To use them you will need the keyword "with".

#### 5.4.1  Send signal

Description:

The best description is fo

und in Apple's scripts:

" To help debugging, "with send

            - signal SIGFPE" will trigger a fake floating

crash the process and show the call stack leading to the offending operation.

For the shipping version "deny" is probably better 

the process. "

There is a special exception, where send

                - signal doesn't apply to mach

It can be applied to allow and deny actions

.

Syntax:

(with send signal SIGNAL)

(deny file read* (w ith send signal SIGFPE))

The target binary will crash with a floating point exception when it tries to read any file.

Floating point exception

                            - point exception,

which will 

#### 5.4.2  No log

D escription:

because it vetoes the operation

without killing 

                            - * operations.


---

## Page 36

v1.0

Do not log denied operations. Applies only to den

y action.

(with no log)

(deny file read* (subpath "/tmp")

(with no log))

#### 5.4.3  No sandbox

Description:

Applies only to allow action and process

              - exec operation.

(with no sandbox)

????

### 5.5  Other keywords

5.5.1 require any and require

These are the keyword

s for logical OR and logical AND.

(require any (filter1) (filter2) ...)

Example:

(deny file read data

(require all

(file mode #o0 644)

(subpath "/private")

(literal "/p rivate/tmp/test2"))

In this case, reading the contents of the file test2 located in /tmp will only be denied if it matches 

all the three filters (the subpath filter is 

somewhat 

cat: /tmp/test2

: Operation not permitted

Log output:

Sep 3 23:27:44 macbox

sandboxd[13401]: cat(13400) deny file

$ chmod 0614 /tmp/test2

aaaaaaaaaa

5.5.2 - trace

This command will assist you in building 

custom profiles for any app.

redundant in this case).

(trace output_file_name)

Example:

(trace "trace.sb")


---

## Page 37

v1.0

This will append rules to the file "trace.sb" (located at the working directory you are executing 

sandbox exec). These rules are for operations that would have been

Then you can use the command sandbox

                - simplify to create a simplified sandbox profile based

this trace file.

Note:

If you want to develop a custom profile from scratch using this feature, you could start with 

something like this:

(version 1)

(deb ug all)

(import "bsd.sb")

(trace "trace.sb")

(deny default)

But this doesn't work as expected. This will indeed generate a trace, but just for the initial 

permissions. Let me explain this with an example. I started using the initial profile described 

abov e:

            - i386 /Applications/iTunes.app/Contents/MacOS/iTunes.orig

arch: /Applications/iTunes.app/Contents/MacOS/iTunes.orig isn't executable

$ more trace.sb 

(allow process

    - exec (literal "/us

r/bin/arch"))

(allow process

    - exec (literal "/usr/bin/arch"))

(allow file read data (literal "/usr/bin"))

(allow file read data (literal "/usr/bin/arch"))

As you can observe, only the initial rules to start the arch process are "traced". The next step is 

add these to the initial profile and continue tracing, in a iterative process.

You might feel it's easier than checking the logs and manually adding stuff, but it's still not an 

automatic process.

**6  - Special hardcoded cases**

The following special cases 

can be found inside the code:

- Allow mach - bootstrap if mach
              - lookup is ever allowed.
- Allow access to webdavfs_agent if file
                  - read* is always allowed.
- Never allow a sandboxed process to open a launchd socket.

**7  - A sample**

**sandbox**

**profile**

This is a working san

dbox for Vienna 2.5.x version. It's still far from perfect but it works for normal 

denied.

usage from what I could test. It is much more granular from other profiles I could find. The 

consequence is a bigger and slightly more confuse profile. It's not an easy task

to build manually a 


---

## Page 38

v1.0

very tight sandbox. It requires a lot of testing and patience!

assist in this process (not used to build this profile).

You will need to replace the string %username% with yours. A global search a

job.

------------ START HERE ------------ nd replace does the 

; Vienna 2.5.x Sandbox profile

; (c) fG!, 2011

; Note: replace %username% with your username

(version 1)

; well this doesn't seem to work...

(de bug all)

;(trace "trace.sb")

; stuff we allow to execute

(allow process

    - exec (literal "/Applications/Vienna.app/Contents/MacOS/Vienna"))

; no need for forks? great :

;(allow process

    - fork)

; it needs to read some sysctl variables

(allow sysctl read)

; where?

(allow sysctl write)

; ----------------

; READ PERMISSIONS

; ----------------

; allow read system libraries and frameworks (from bsd.sb)

(regex

#"^/usr/lib/.*

\.dylib$"

#"^/usr/lib/info/.*

\.so$"

#"^/private/var/db/dyld/"

#"^/System/Library/Frameworks/*"

#"^/System/Library/PrivateFrameworks/*"

#"^/System/Library/*"

The trace directive is very helpful to 

; reverser@put.as

; Vienna Frameworks


---

## Page 39

v1.0

(allow file read*

; Vienna itself

#"^/Applicat ions/Vienna.app/*"

; Growl

#"^/Library/PreferencePanes/Growl.prefPane/*"

; allow read to required system stuff

(allow file read*

#"^/usr/share/zoneinfo/*"

#"^/dev/*"

#"^/usr/share/icu/*"

#"^/private/var/folders/*"

; do we really need access to keychains ?

#"^/Users/%username%/Library/Keychains/*"

#"^/Library/Fonts/*"

#"^/Users/%username%/Library/Caches/*"

#"^/Users/%username%/Library/InputManagers/*"

; what's this ???

#"^/private/var/db/mds/system/*"

(literal "/private/etc/localtime")

(literal "/Users/%username%/Library/Preferences/com.apple.security.plist")

(literal "/private/var/db/mds/messages/se_SecurityMessages")

(literal "/User s/%username%/Library/Preferences/com.apple.systemuiserver.plist")

(literal "/Users/%username%/Library/Cookies/Cookies.plist")

(literal "/Users/%username%/Library/Preferences/com.apple.LaunchServices.plist")

(literal "/Users/%username%/Library/P

references/pbs.plist")

(literal "/etc")

(literal "/Users")

(literal "/Users/%username%")

(allow file read metadata

(literal "/")

(literal "/var")

(literal "/Applications")


---

## Page 40

v1.0

(literal "/System")

(literal "/Users/%username%/Library/Preferences")

(literal "/Library")

(literal "/Users/%username%/Library")

(literal "/Library/PreferencePanes")

(regex 

#"^/Users/%username%/Library/Autosave Information/*"

; allow read application data

(allow file read*

(regex

#"^/Users/%username%/Librar

y/Application Support/Vienna/*"

; allow read to preferences files

(regex #"^/Users/%username%/Library/Preferences/ByHost/.GlobalPreferences.*")

(literal "/Users/%username%/Library/Preferences/.GlobalPreferences.plist")

(literal "/Users/%username%/Library/Preferences/uk.co.opencommunity.vienna2.plist")

(literal "/Library/Preferences/.GlobalPreferences.plist")

; web browsing related

(allow file read*

( regex

#"^/Users/%username%/Library/Icons/*"

#"^/Users/%username%/Library/Internet Plug

#"^/Library/Internet Plug

                - Ins/*"

; still missing some? well we could even remove quicktime and java :

(literal "/Users/%username%/Library/Preferences

(literal "/Users/%username%/Library/Preferences/com.apple.java.JavaPreferences.plist")

(literal 

"/Users/%username%/Library/Preferences/com.apple.quicktime.plugin.preferences.plist")

                        - Ins/*"

; -----------------

; WRITE PERMISSIONS

; -----------------

/com.github.rentzsch.clicktoflash.plist")


---

## Page 41

v1.0

; allow write to dtrace related stuff

(allow file write* file ioctl

(regex

#"^/dev/dtracehelper$"

(allow file write*

(regex

#"^/Users/%username%/Library/Application Support/Vienna/*"

#"^/Users/%username%/Library/Caches/*"

#"/Users/Shared/SC Info"

#"^/Users/%username%/Library/Cookies/*"

#"^/private/var/tmp/tmp.*"

#"^/private/var/folders/*"

#"^/Users/%username%/Library/Preferences/uk.co.opencommunity.vienna2.plist*"

; web browsing related

(allow file write data

(literal "/Users/%username%/Library/Icons/WebpageIcons.db")

(allow file write*

(literal "/Users/%username%/Library/Icons/WebpageIcons.db

; ----------------

; MACH PERMISSIONS

; ----------------

                            - journal")

(global name "com.apple.SecurityServer")

(global name "com.apple.dock.server")

(global name "com.apple.distributed_notifications.2") (allow mach lookup (global name #"^com.apple.bsd.dirhelper") (global name "com.apple.system.logger") (global name "com.apple.system.notification_center") (global name "com.apple.CoreServices.coreservicesd")


---

## Page 42

v1.0

(global name "com.apple.audio.coreaudiod")

(global name "com.apple.audio.systemsoundserver")

(global name "com. apple.metadata.mds")

(global name "com.apple.ocspd")

(global name "com.apple.SystemConfiguration.PPPController")

(global name "en (Apple)_OpenStep")

(global name "com.apple.system.DirectoryService.libinfo_v1")

(global name "com.apple.system.Direc

toryService.membership_v1")

(global name "com.apple.windowserver.session")

(global name "com.apple.windowserver.active")

(global name "com.apple.FontServer")

(global name "com.apple.pasteboard.1")

(global name "com.apple.tsm.uiserver")

(global name "com.apple.SystemConfiguration.configd")

(global name "com.apple.VoiceOver.running")

(global name "com.apple.FontObjectsServer")

(global name "com.apple.FSEvents")

(global name "com.apple.cvmsServ")

(global name "GrowlApplicationBridgePathwa

; ------------------------------

; MEMORY AND NETWORK PERMISSIONS

; ------------------------------

(allow ipc posix shm)

; network related stuff

; add other ports if needed

(allow network

    - outbound

(remote tcp "*:80")

(remote tcp "*:443")

(remote unix socket (path literal "/private/var/run/mDNSResponder"))

(allow system

    - socket)

(deny default)

------------ END HERE ------------


---

## Page 43

v1.0

- Where to find more information

These are some links and books where you can fi

Enterprise Mac Security book has a chapter dedicated to just this.

Books:

Enterprise Mac Security: Mac OS X Snow Leopard, 

Hunter, Gene Sullivan

The Mac Hackers 

Handbook, by Charlie Miller, Dino Dai Zovi

Websites:

http://www.sedarwin.org/

http://www.romab.com/ironsuite/SBPL.html

http://www.tomsick.net/projects/sandboxed

                - safari

http://tengrid.com/wiki1/index.php?title=Sandbox

Chromium/Chrome sources

### Appendix A

All available operations

(you can use this info with the IDC script presented in the next appendix)

------------ START HERE -----------default

file*

file chroot

file ioctl

file read*

file revoke

file write*

| - write | write - owner | - owner   |
|---|---|---|
| - write | write - mount | - mount   |
| - write | write - mode | - mode   |

ipc*

nd further information about this technology. The 

By Charles Edge, William Barker, Beau 

ipc posix*


---

## Page 44

v1.0

ipc sysv*

mach*

mach bootstrap

mach lookup

mach priv*

network*

network inbound

network bind

network outbound

process*

process exec

process fork

signal

sysctl*

sysctl read

sysctl write

system*

system acct

system audit

system fsctl

system nfssvc

system reboot

system socket

system swap

job creation

------------ END HERE ------------

**Appendix B**

The IDC script I used to find the correspondence between

module MAC framework hooks

( in sandbox kernel module/extension)

SBPL operations and sandbox kernel 

. It's a quick hack but it gets 


---

## Page 45

v1.0

the required information (although with some dupes and not relevant information).

dumped to IDA console 

(you can mo dify to write to a file, if you wish).

This code is a bit lazy because I used only the IDC command console, which doesn't support 

functions. It's not so important that deserves better code 

------------ START HERE ------------

#include <idc.idc>

auto ea, crap;

auto file, fd, first, register1, register2;

auto myarray, myarrayid, myindex, operation,kr;

// ask where the file with operations names is

// the file we load contains all operations names, check Appendix A

file = AskFile(0, " ", "Load the file wi

th all operations names:");

// load the array with the operations names

myarray = CreateArray("operationsnames");

// if it doesn't exist, then load information

if (myarray != 

Message("Array doesn't exist; loading data...

myarrayid = GetArrayId("operationsnames");

myindex = 0;

Message("File open error!

// read the contents into the array

while ((operation = readstr(fd))>0)

Message("Operation %s", operation);

kr = SetArraySt

ring(myarrayid, myindex, operation);

myindex++;

Output is 

// get the first location that calls for _cred_check

ea = RfirstB(first);

// get the address before the call so we can ve

rify the parameter being passed

// TAKE CARE OF CRED_CHECK

//first = 0x253E;

first = LocByName("_cred_check");


---

## Page 46

v1.0

// which is the information we are looking for

// get an array id so we can work with the array

myarrayid = GetArrayId("operationsnames");

e stuff we want

while (ea != BADADDR)

// retrieve the register

// verify if it's edx, the operation we want is mov edx, #VALUE

while (register1 != "edx")

// try to find the right operation... not exactly the best 

// retrieve the value

register2 = GetOpnd(crap,1);

// output the values we want, we are not writing anything to a file, just to screen

/ / we can copy & paste later (or you can modify script to write to a new file)

Message("%s;%s", GetFunctionName(ea), GetArrayElement(AR_STR, myarrayid, 

xtol(register2)));

// iterate to the next call...

crap = FindCode(ea, SEARCH_

// CRED_CHECK_SOCKET

//first = 0x2596;

first = LocByName("_cred_check_socket");

ea = RfirstB(first);

while (ea != BADADDR)

while (register1 != "edx")

algorithm heehhe

register2 = GetOpnd(crap,1);


---

## Page 47

v1.0

Message("%s;%s", GetFunctionName(ea), GetArrayElement(AR_STR, myarrayid, 

// CRED_CHECK_VNODE

//first = 0x24D2;

first = LocByName("_cred_check_vnode");

ea = RfirstB(first);

while (ea != BADADDR)

while (register1 != "edx")

register2 = GetOpnd(crap,1);

Message("%s;%s", GetFunctionName(ea), GetArrayElemen

// sb_evaluate

//first = 0x34DC;

first = LocByName("_sb_evaluate");

ea = RfirstB(first);

SEARCH_CASE | SEARCH_NEXT);

while (ea != BADADDR)

while (register1 != "dword ptr [esp+4]")

t(AR_STR, myarrayid, 

register2 = GetOpnd(crap,1);

Message("%s;%s", GetFunctionName(ea), GetArrayElement(AR_STR, myarrayid, 


---

## Page 48

v1.0

------------ END HERE ------------
