---
layout: post
title: "Exploring cross-device application development"
author: "Jakob Meier"
categories: Blogging
tags: [rust, api, architecture, distributed-systems]
image: 21/wip.jpg
image_tooltip: "todo"
# thumbnail_style: no-crop
lang: en
ref: new-process-model
techs:
---

<p class="intro">
Today, I write about a way I would like to develop user facing applications that
differs from what I see today. All examples are in Rust and Rust would be my
first choice to implement the proposed framework. That said, the ideas are not
tied to Rust and you don't need Rust knowledge to understand this article.
</p>

The intention of this article is to articulate my thoughts properly and share it
with others and learn about their opinions. I hope you find it an interesting
read and maybe even have some responding thoughts to share with me.

## TLDR

*Quick summary:*

*I imagine a cross-device app framework that let's me span an application across
devices as easily as spanning it across CPU cores. Ideally with real-time,
peer-to-peer synchronization between devices. Then let me install such
cross-device apps in a sandboxed environment where they can't access the
Internet for anything other than internal synchronization. Here is a [demo] that
let's you run an app across multiple browser tabs on any device.*


## Part 1: Introducing the Idea
<!-- This part is an incomplete draft v0 -->

### Modelling Operating Systems as Distributed
At university, I came across [the Barrelfish OS][barrelfish]. This research
operating system has produced many publications and theses between 2008 and 2020.

One of the publications has the title [_Your computer is already a distributed system. Why
isn’t your OS?_][hotos09] (2009) by A. Baumann et al. which is a great starting point.

The argument is, roughly speaking, a multi-core CPU has many of the properties
of a distributed systems, such as node heterogeneity, partial failures, and
latency. Each CPU core is a node in this analogy.

Unless you have a deeper understanding of how CPUs are architected, you may not
find this argument very clear. But if you have learned about CPU cache
coherence, you might remember how cores have to exchange messages between each
other to guarantee memory consistency. For example, if a memory slot is updated
by core 1 and core 2 is running a thread in the same address space, it needs to
know about this update.

Interestingly, this is implemented using multi-layered network protocols
internal to the CPU, comparable to the [OSI model][osi] taught in networking
classes. And this is what really makes a CPU look like a distributed system of
CPU cores to me personally.

The genius of this architecture is that memory looks coherent to user
application developers. Sometimes we have to define the
[`std::sync::atomic::Order`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)
for memory accesses to tell the OS what guarantees it should ask from the CPU.
But generally speaking, it just works. The inherent complications of distributed
systems are hidden.

The authors of aforementioned paper go on to say that operating systems might
benefit from an architecture where they embrace the distributed nature instead
of relying on hardware coherence too much.

The proposal, implemented in Barrelfish OS, is to run independent kernel nodes
on each core. Global state can be maintained by sending messages between cores
following distributed systems patterns. For example, a system call for memory
mapping (e.g. [mmap]) would cause a two-phase-commit in the underlying
distributed operating system. But for the developer of user-level code, it is
still just a memory mapping system call.

Flipping the idea on it's head, we can also think of the OS as a distributed
peer-to-peer application and each core runs a client. These client run a
CPU-driver, which can be optimized for the type of core, for example performance
vs efficiency core.

This architecture is explained more in [_The Multikernel: A new OS architecture for
scalable multicore systems_][sosp09] (2009) by mostly the same authors. The main
argument for the multikernel architecture seems to be that the cache coherence
abstraction will become too costly at some point.

Now, I am not here to discuss performance today. However, the distributed OS
architecture and the techniques to make it work are the foundation for how I
envision cross-device applications. Specifically, given an OS can be designed as
a distributed system and hide the complication to application code, why not
build a distributed application runtime that is as simple to use a
non-distributed runtime?

Okay this is was quite a tangent but I think it helps to understand where my
idea is coming from. The remainder of the article will be less about operating
systems and more about application runtime abstractions. With the goal to design
an architecture which lets us run, for example, a code editor on multiple
machines at the same time without worrying about the distributed nature. Think
compiling on you arm based MacBook and running the code on an x86 machine in the
cloud. Or drawing on your tablet and rendering on your workstation. All without
specialized synchronization code by the application but rather have it
abstracted in the runtime framework.

### WASM: The Modern King of Portable Code

I have been throwing around the term "application runtime framework". By which I
mean a general-purpose library which manages how applications are run. This
includes starting, stopping, and scheduling arbitrary code.

To understand a specific runtime framework, one key question is what standards
the code in question needs to fulfil. Operating systems are also application
runtimes, among other things. In the case of Unix based systems, the code must
adhere to the [POSIX standard][posix], while Windows has [its own
standards][win-api]. Crucially though, both are designed to run on a single
machine and it's not clear how to extend it to multiple machines.

For _frameworks_ that build on top of operating systems, we have [Tokio][tokio]
as a good example. Here the interface is tied to the Rust language and its
[`Future`][future-rs] type. But again, an instance of tokio is bound to a single
machine. If you wanted to make it schedule across many machines, that would be
hard. You would have to figure out how to send futures across the network.

I want a runtime API that makes no assumptions about the exact device the code
is running on. Here, [WebAssembly (WASM)][wasm] comes into play. Its (upcoming)
[component-model](https://github.com/WebAssembly/component-model) is a great
start for a cross-device runtime.

In the component model, a WASM module specifies its interface in the [WIT
language](https://component-model.bytecodealliance.org/design/wit.html). In this
space, the term [_WIT World_](https://component-model.bytecodealliance.org/design/worlds.html)
is used to describe the APIs the WASM code needs from the outside and what API it provides.
A world is also used to define the capabilities of a host, much like POSIX.

This naturally leads to a runtime in which applications are defined by a set of
components and their WIT worlds.

But why stop there? In my distributed vision, we would have multiple devices on
which the runtime can schedule the component to run on. We can now also describe
the device capabilities as WIT worlds. For example, a phone has a touch input
API, it has access to a front and a rear camera, acceleration sensors and so on.
A workstation might provide access to a GPU. And both provide an API to display
items on screen.

A smart-enough, distributed runtime can match components to devices and
magically glue calls between the components even if they have to go over the
network.

Now, this relies on WASM being completely portable. Which it is by design, at
least on the WASM execution layer. Things become more complicated when looking
at the component model. The commitment to a [Canonical ABI][wasm-abi] already
solves many potential problems. But if there was shared state between
components, that would also cause issues for our runtime. Luckily, it seems that
the [shared-nothing approach][shared-nothing] will be the initial assumption for
the component model, with other designs only listed as [Future
Features][shared-something].

This means it should even be potentially cheap to migrate components between
devices, or even replicate the state in global state, if a component doesn't use
much linear memory.

*TODO: Also write about state sync? Or should it go in the implementation part only? Then maybe tease it here?*

### Motivation for a Distributed Application Runtime

I believe such a runtime could potentially deliver a better developer experience
and a better user experience compared to how applications are developed today.
For example Apple is well-known to provide cross-device experiences as long as
you buy all products from them. I believe several Android phone vendors are
catching up, too.

But there is one project which seems to incorporate everything I've mentioned so
far. [HarmonyOS][harmony-os] uses a multikernel architecture and has a concept
of so-called _Super Devices_ to easily share media devices like screens and
speakers across phone, laptops, tablets and so on. I was really excited about
this when I hear the first rumours but got rather confused when it finally
launched in 2019. Communication was quite unclear and there were simply no
devices available in my area that are supported.

HarmonyOS has matured a lot since then but I am still not really convinced.
Perhaps for lack of knowledge and understanding of how it works. But in my
current state of ignorance, I believe it is just yet another vendor-locked
ecosystem. Which makes me sad, since their technology stack looked to me like it
was designed to run on heterogenous nodes that could be even a browser session,
as opposed to an app that was built from ground up with their SDK.

What would be better? A Rust crate that allows to selectively sync some state
between machines. One where you can dynamically register _super device_ on one
instance of the app and use it in other instances. And the main binary could run
as WASM in the browser or natively on any given device from iOS to Windows. Only
the code that you want to run in distributed fashion would need to be WASM.

But there are more benefits a framework could give. 

*TODO: local-first approach?*

*TODO: remove references to network isolation*



## Part 2: Testing the idea
<!-- This part is in draft version 0 -->

The first step to validate if this idea has any merit is to create a small toy application that demonstrates the principles.

I sort of did that in this [demo] which you should be able to try out.
It demonstrates real-time UI synchronization and cross-device work scheduling across your local network. It doesn't work if you are in different networks.

The demo can render a static scene using CPU ray-tracing. The ray tracer is
mathematically based on what Peter Shirley, Trevor David Black, and Steve
Hollasch teach in [_Ray Tracing in One Weekend_][rt-one-weekend] and a bit of my
own creativity sprinkled in. The (poor) implementation in Rust can be viewed at
[jakmeier/distributed-wasm-rt-demo/clumsy-rt][clumsy-rt-src].

Click the start button and a CPU ray-tracing workload will start. Click the
button again and the same picture will be rendered again but with higher
quality. Repeat it a few more times and you will notice a considerable slowdown
in rendering time.

If it works as intended, you should see something like this.

![A screenshot of the demo. It shows a rendered scene on the top, start and stop
buttons on the bottom left and a worker thread management view on the bottom
right.](/assets/img/23/distributed_wasm/demo_home.png)

So far, everything lives in the isolated browser tab as usual. But I want to
show how to make this toy application run across two devices at the same time. 

For this, open the [same link][demo] on another tab, or ideally on a different
device in the same network. (It won't work if one of them is in your local Wifi
but the other is in a cellular network.)

Then you can establish a peer-to-peer connection between the two devices.
Switch to the network tab using the menu at the bottom.

![Show where to click on the network tab.](/assets/img/23/distributed_wasm/click_on_network.png#center)

A random ID should appear. Press "Find Peer" on device 1 first. It will now connect to a signaling server that I'm hosting. (Yes, connection establishment requires an Internet connection and relies on this server. It's not fully local in this demo.) The signaling server is a simple service that forwards messages between two parties with a matching id. Review [the full code on GitHub](https://github.com/jakmeier/distributed-wasm-rt-demo/blob/4a15de702df1a70086d245ea35b7f84280d3c0d4/webrtc-signaling-server/src/main.rs).

On your second device, replace the random id generated on this device with the
id from the first device. Press "Find Peer" and wait. If both devices are in the
same network, you should see a successful connection within at most a few
seconds. Once it's fully connected, you should see the message "Connected" on
screen. When this happens, the WebRTC connection through local network has been
established and the Internet connection to the signaling server is no longer needed.

![Connection successful screen.](/assets/img/23/distributed_wasm/p2p_success.png)

Once the connection is established, you may go back to the main page and press
render again. The workload is now shared between both devices, each using 4
local worker threads.

Ray-tracing is a workload about as embarrassingly parallel as possible. Hence it
is easy to share between devices. Add more workers on one tab and you will see
it takes over more of the work by spawning more local threads. The message
"Total Compute: X.x s" refers to how much compute time has been spent on this
device or tab only, you can use it to compare how much work was done by each
device.

![Two screens side-by-side with unequal amount of
workers.](/assets/img/23/distributed_wasm/work_distribution.png)

Ray-tracing can also easily be scaled up and down in complexity, with obvious
 improvements in the output quality. Feel free to play around with different
render quality settings if you go to the settings tab.

![Connection successful screen.](/assets/img/23/distributed_wasm/settings.png)

This settings tab is where the synchronized UI comes into play. Open it on both
devices and you will see it synchronizes instantly. Because everything goes
through the local network, it only has a few milliseconds of a delay, which
should be virtually impossible to notice by humans.

But this also allows you to change the setting on one device (e.g your phone)
and keep the main screen open on the laptop. I think this is the real use case
here, dynamically sharing the UI across devices and make it work as if it's on
the same device.

### External workers

To take the idea of sharing work across devices a step further, I also
integrated the ray tracer as a service into a WASM component. For this, I tried a
few different frameworks but [Fermyon Spin](https://www.fermyon.com/spin) ended
up as the easiest to use right now. Think ot it as AWS' lambda for WASM
components.

If you click on "Fermyon Cloud" it will connect to such a component in the cloud
and offload some of the ray-tracing work to it. I restrict it to a single
connection to Fermyon Cloud to not overload my plan on Fermyon cloud. But you
can very easily run spin locally and have it serve my component. Just follow the
instructions in [this README][spin-component]. You can then spawn as many
connections to you local component as you want.

![A mix of workers as shown in the web view.](/assets/img/23/distributed_wasm/workers_mix.png)

This allows to share work between your client(s) and servers. In my demo, this
might seem like just a gimmick. But in the modern era with more and more
specialized hardware, I think this is legitimately useful. If your phone has an
[AI accelerator][npu] it might make sense to do small work locally, while large
work items should go to a big server somewhere in a datacenter. This could make
the interface for either option one and the same.

That's all for the fancy demo. Let's look into how this was built.

## Part 3: Implementation
<!-- This part is just a very rough outline and may require major restructuring -->

In this last part, let's go through a few interesting components a framework for
cross-device apps would need and what challenges I identified in my
experimentation so far.

### The network stack

For our framework, we need to establish connectivity between the devices and also provide a communication layer to the apps running within the framework.

#### Pairing

- who to connect to and how?
- in my experiment, I went with [WebRTC][webrtc] because that's an established standard that works in all major browsers
 - candidate addresses need to be shared between both devices
 - QR scans could work here, some people have done it through sound (TODO: source)
 - simple tag ("password") and small connection establishment server did the trick in my case (not full local-first!)
 - hence, I start with a WS connection to the NTMY helper, then messages are forwarded
 - "upgrades" to WebRtc connection + datachannel
 - challenge: peer-to-peer is not always possible, unless you count TURN as p2p (framework could include such a service, would probably need to be paid for) (VPNs can also be tricky?)

#### Application layer networking
- somehow users on one end must be able to send messages and they are received by the peer
- I'm using a pub-sub pattern that allow to do `share<T: Any>(T)` and `listen<T>(Fn(T))`
- The framework takes care of sharing these messages across devices (in the demo, only a specific type TODO is synced, everything else remains local)
- seems like a perfect case for serde, somehow I ended up implementing it by hand (to work with blob & arraybuffer directly)

#### P2P mesh
- my demo assumes only one peer, no failures
- a framework would need to solve this / build on top of libraries that have solved it already

### UI
- input on multiple devices: different views, resolutions, hardware sensors
- demo: abstract over pointer events (touch/mouse) and use virtual resolution (also: don't share UI events, share logical events)
### Application Manager
- prepare runnable code
  - WASM -> WebWorker
  - Rest API to non-web sources of compute (talk about spin)
  - the framework should be more dynamic
  - component model: define interface (universe) and send WASM code over the network instead of statically prepared
- schedule work
  - I used per-device queue with work stealing: idle devices let other devices know that they have N idle threads
  - framework would ideally be more aware of preferences where code should run ideally (don't mine bitcoin on my phone, but maybe use the NPU on my phone)

## Conclusion

I don't have specific plans to actually implement this framework in the near
future. But I would love to try the architecture on a project that maybe a bit
closer to reality. With more application-specific learnings, maybe just mabye,
at some point it might make sense to generalize it into an actual framework.

But that's all for now. I hope there were some new learning or ideas for you in
it. In any case, I am open for feedback, either on reddit[TODO], on
github[TODO], on near.social/near.org[TODO], or drop me a message on
inbox@jakobmeier.ch. Thanks for reading!

# Old notes, still useful for fleshing things out in more details

<!-- ## What is wrong with user applications today

First, on the high-level, why would I say that is sucks? Mostly because it has been unchanged since decades. 
Everything network related is still based on top of [Berkeley Sockets] first released 40 years ago.
What happened to the fast moving IT sector? It seems we are just changing themes rather than the underlying system.

At the heart of it is the client-server model, where some code runs on the user's device, like the browser that rendered the HTML for you to read this article. And the server, which safeguards the data on a server.

This setup typically makes the client useless without a network connection to the server. This leaves you with a strong dependency on the software vendor long after you installed it. Plus, it severely hinders your experience when on a network with poor bandwidth, latency, or error rate.

The rigid client-server model also limits the experiences developers can create. In the all-connected world we live in today, why is my phone not able to talk more freely with my laptop? Or my phone with your phone? Back in the days, we sent messages from one phone to another through bluetooth or even [infrared communication]. Today, the easiest way for me to share a file with you is to exchange phone numbers or social media tags and send it to you through a messaging app. Exactly the client-server model, I upload it to a server first so that you can download it from the server.

I acknowledge that this is far more convenient than an IR transmissions where we both hold our phones pointed at each other. But we are comparing a very old technology which I would have expected would become more convenient to use if iterated on for 20 years.

So what would I want? I would want to touch our phones together and confirm on a pop-up that I want a direct peer-to-peer connection between the two phones. I don't care about the protocol used, nor the frequencies at which the data is sent. Just give me this simple user experience and send the data from where I have to where I need it directly. No need to upload it all to our tech overlords, passing through a few dozen middleware boxes each consuming electricity and adding delay to my interaction.

But this is just one example. I am trying to make a general point about how client-server applications are not always the best fit. So let me give you a list of others examples.

- When I take a picture on my phone, the best way to have it on my laptop is to upload it to cloud storage. (Google Drive, iCloud, OneDrive, Dropbox etc)
- After I get home from a run, my Garmin smartwatch needs to upload the data to the cloud before I can view it in the Garmin Connect app on my phone. Btw, the uploads happens through Garmin Connect. But without internet connection, I cannot look at the activity because it only displays the data it download from the server.
- Let's say I am reading an article on my laptop, then I leave the house and would like to continue reading on my phone. At least modern browsers allow to "share" tabs between devices. As far as I can tell, all implementations of this are essentially glorified bookmarks stored in the cloud. In other words, they upload the currently opened URL to a server and let you download it on another device.
- I install an app on my phone. My laptop can't interact with it in any way. -->

<!-- ## The reasons why we are stuck with this model

This is the reality I got used to. But it could be better. Imagine a constant connection between your devices. Why is the application state not constantly shared and synchronized between my devices? And with synchronization, I don't mean everything is uploaded to the cloud and ready for download. No, it should be peer-to-peer between my devices, the data never leaving my house and not requiring an internet connection.

Of course, I am not the first to rant about this topic. Indeed, the [Local First Cooperation][local-first] has described the problem of relying on servers in more detail and more professionally than I am doing here. But I want to go a step further and challenge the idea that an application is limited to a single device. I believe it would be better, for developers and users alike, to think of an application as something that spans multiple processes running on different devices, sharing a single distributed state. I couldn't find much literature or activity on this front, please point me in the right direction if you know of something that already exists.

Another way to think about it as a user is this: I have a dual monitor setup for my desktop PC. When I drag and drop a browser window from one screen to the other, this is instant and doesn't need to reload the page. Why can't I drag it to my phone?

I believe the answer has something to do with how operating systems have been working for the last 50 years, give or take. Somehow an operating system can flawlessly use multiple screens and even share them safely between multiple applications. But it cannot handle to do it across multiple devices. We run completely independent OSs on each device and each process is locked into one such OS. Sure, the OSs can communicate through the network but it's up to the developer of the app to do it, there is no built-in way that automatically shares state across devices or something like that. -->

<!-- ## Alternative application model proposal

Maybe it's time to reconsider the device - OS - application mapping constraints. An application, in my opinion, should be able to be running on multiple devices at the same time. It may be displayed on multiple screens, which could be duplicated of each other or offer different views per device depending on the use case. Ideally the OS should take care of state sharing, much like it does when we run multiple threads within the same process today. 

Unfortunately, the term OS is a bit of a misnomer these day. It has been overused for many things and depending on your background it will mean something else to you than to your neighbor. To make things worse, the things I'm about to propose challenge what an OS boundaries are and adds more layers to it. So to avoid misunderstandings, I will try to name the relevant components directly whenever possible. -->
<!-- and use OS vaguely as the thing that combines all these components even across devices. -->


### Application Manager

Installing and starting applications is at the heart of what we want to do with our devices.
The change has to start here.

The fundamental understanding of what an application is from an OS perspective should change from "a group of threads running on this device" to "a group of processes and threads running on many devices".

So, if I install an app, I don't install it on my phone or on my laptop. I install it in my "OS" and it is instantly available on all my devices to the extent that this app can work on that device.

As an example for using such an app, when I start scrolling through my favorite messaging app on my phone, I could see the attached images on the larger screen of my laptop. All in real-time, there is no excuse why it should take longer to display it on my laptop than it takes to show it on my phone. Or when I open image editing software on my desktop, maybe I want to have a tablet or phone to draw with a pen. I think this is already fairly standard today, but it has to be implemented by each application anew. Or users can hack their way around ot by using some screen mirroring tools. But in my opinion, in 2023 we should demand this natively as part of the OS.

On the technical side this means code has to be written to allow running on multiple devices.
There are already byte code formats and scripting languages that run on just about any platform, we just have to put it to work. My first choice would be WASM bytecode as the executable format, which I also use in the demo.

### Scheduler

Let's start with the scheduler that traditionally assigns threads to CPU cores. It will keep doing that but on top also assign processes to devices. There might be constraint to which threads can run on which device, such as a 3D game's rendering thread will want to be on the same device as the strongest GPU available, but I believe most of the code could run on any device.

### Device drivers

Hardware drivers should still run on each device independently. But the layer that let's you share hardware between processes (two applications display something on screen at the same time) should be expanded to include to work across the device boundary.


User input is another important piece. As a developer, I want instant, real-time knowledge of the touch screen inputs of my phone on my desktop PC. In fact, the code I write should not make any assumptions on the input. Maybe it's touch, maybe it's a cursor, maybe it's gesture recorded by a VR setup. My code should ideally handle all of those. The "UI component" of the OS will provide the inputs.

### Network

An interesting case of "hardware" an OS manages is the network stack. My phone and my laptop use different IPs, at least in the local network. But an application running on the two devices simultaneously doesn't care about that fact. So let's abstract away the IP based communication away and from the application developers view. They just get incoming data packages, which may come from the phone's or the laptop's internet connection. The OS's network component does the syncing. It has been doing it for decades between different CPU cores on the same device, stretching it to contain other devices isn't much of a conceptual jump.

Furthermore, if the OS understands the concept of an application spanning multiple, then the application permission component can also take advantage of it. Wouldn't it be nice to have a file sharing drive app on your phone and allow it to only sync your files between your devices, but otherwise prevent it from accessing the network at all? The OS could do that for you.

## What software engineers can do today

"But Jakob", I hear you, the hypothetical reader in my head, ask: "How can you propose to make such drastic changes to the operating systems of today? That will never happen."
You would be right to point this out. As I see it, these are fundamentally tasks an OS is supposed to be doing but it can also be built in normal libraries on tap. And it should be built as libraries first for experimentation. If a form of such a new application model is getting real world traction, the native OS support will follow. And eventually the hardware support (think MMU for CPUs).

So this leaves me in the spot where I have to build this library to proof my point, right? Well, I'm not in the illusion that such a library would be an easy or small task. So let's shelve that idea. But perhaps I can implement a toy application as demo, that feels like this library exists already? Indeed that's what I have done.

The demo is available at TODO. It does... and you can ... Please try it out. The rest of the article quickly goes over the main challenges for implementing it using today's technology stack.

# Demo Time

## The Demo: A CPU ray-tracer that shares the workload across devices

Click the start button and a CPU ray-tracing workload will start. Click the button again and the same picture will be rendered again but with higher quality. Repeat it a few more times and you will notice a considerable slowdown in rendering time.

You can now create a peer-to-peer connection between two devices if you switch to the network tab using the menu at the bottom. A random ID should appear. Press "Find Peer" on device 1, then copy the ID from device 1 into the field of device 2 before you press "Find Peer" on the second device. If both devices are in the same network, you should see a successful connection within at most a few seconds. If you are not in the same network, connecting the second device to a hotspot of your phone can be an easy way to ensure they are on the same network.

Once the connection is established, you may go back to the main page and press render again. The workload is now shared between both devices, each using 4 local worker threads. You can also play around with different render quality settings if you go to the settings tab.

The ray tracer is based on what  Peter Shirley, Trevor David Black, and Steve Hollasch teach in [00_Ray Tracing in One Weekend_][rt-one-weekend]. But I am taking shortcuts by using [ncollide3d][ncollide] for the ray-object-collision checks. 

By default, the rendering uses 4 worker threads in the browser. Click on "Web Worker" to add another thread on this device. If you click on "Fermyon Cloud" it will connect to a WASM component. And finally, if you click on "Localhost", it will try to connect to an address running on the same machine as the browser session. More on that later.

## Implementation challenges

### Universal code
Ideally write it once, compile it once, run it everywhere. Rust + WASM almost gets me there. [Fermyon/spin](https://github.com/fermyon/spin) let't me run the same WASM component on my machine and also in the cloud. This could also work in the browser but for the demo I wrote a custom JS wrapper and it requires separate compilation using [wasm-pack] and [WebPack] to bundle it as a dependency in my frontend. Ideally, it would be the other way around: the frontend is just one thread in the application which gets scheduled once per display.
I'm super excited about the progress around in the [WASM component model][component-model] which will make this much more straight forward.

### Syncing state
Lots of custom glue code to ensure when an event triggers, such as adjusting a slider in the UI, this gets forwarded to the peer. It wouldn't seem too hard to generalize this and make it happen automagically. I am thinking an event based pub-sub framework that manages data coherence for the app developer much like a CPU manages data coherence between CPU core caches could work well. But there are several other ideas that have already been proven to work for [Local First Cooperation][local-first] software. [Conflict-free replicated data types][CRDT] are useful in this context. Or to keep it high level, the [Actyx] engine (yes, actYx with a Y, not an I) could give you durable event streams and synchronize them through p2p connections out of the box.

### WebRTC signalling
QR codes sound nice but not great because it needs to go both ways, a rendez-vous server was used in this demo

### More than two peers
Connecting two peers is only the hello world of p2p networking. I limited my demo to just two peers because things get more complicated with 3+ peers. Also, I don't handle leaving nodes at all.
Again, the [Local First Cooperation][local-first] is far ahead of me and there are solutions already out there. I found a project called [CRATE][CRATE] from 2015 that implements a collaborative local-first text editor. It was created as part of the paper [An Adaptive Peer-Sampling Protocol for Building Networks of Browsers by B. Nédele et al.][spray-paper]

### Connecting across NATs
Sadly, middleboxes make IP addresses not as global as I wish they were and firewalls often prevent us from easily punching through this. This demo uses STUN to for a best-effort p2p connection but refuses to use TURN, which is again sending all data to a server to forward it rather than direct p2p communication. Hence it most likely won't work if your phone is connected to the internet through a cellular network but your laptop is on wifi. A VPN on one device will also prevent it from connecting to the other.

### UI
An app running across multiple devices also needs a cross-platform UI.

UIs in the Rust community are a hot topic anyway. I don't want to get into it here too much but I believe there is enough demand for it that there will eventually be a Rust crate that works perfectly on all major platforms and meets the need for most use case. (Any maybe there already is, excuse my ignorance if I missed your project, I really don't follow this too much.)

For my demo, I went with a web UI, allowing it to run in all browsers but not natively on any platform. For creating the web app, I used what's best described as my-pet-project-wasm-game-engine, [paddle]. If I wanted to build serious web apps, I would still use JS for the UI elements, to be honest. But I find it cool that I *can* use virtually exclusively Rust code to manipulate the DOM, render to a canvas and so on. So that's what I did.

--------------------------------------

Today, I draw you an image of a potential future user application architecture that spans all your devices seamlessly.
A live demo with Rust code compiled to different WASM backends shows what I mean.

# Introduction
[Condense the blow into 3 paragraphs]
- 1959: Users are crunching more numbers faster than ever. Primary goals: cheap & correct. Single machine, multiple programs in sequence.
    - Time sharing for multiple programs with address space protection through limit registers, a "Director" program to orchestrate it. [https://archive.org/details/large-fast-computers]
    - solved problem: keeping computers busy despite speed mismatch between arithmetic units and I/O. (also accidentally solves interactivity but that wasn't the goal back then)
    - This was a new idea, today we would call the Director a kernel, or perhaps firmware. At the time, a critique was, it would be too slow to do in software.
- today: (tech)
  - MMUs for address isolation (provides VA abstraction)
  - Threads to have concurrent computations on the same address space & sharing resources (allows using multiple CPU cores in parallel)
    - thread = unit of execution (scheduling)
    - thread context: user stack, kernel stack, registers, thread environment block, link to shared process with memory (code/data/heap)
  - Windows API: job objects to group processes and limit/kill them together
  - POSIX: group id for killing forked processes, cgroups (and others) for resource limiting
  - More popular these days to group processes: docker containers
- today (server)
  - usage pattern: multiple machines, many processes
  - tech: communicating through sockets, cluster management software, like Kubernetes
- today (end user usage)
 - People record, edit, and share GBs of media
 - Users use multiple devices and expect great interoperability (shared drive, steam cloud, ...)
 - Data is personal and expected to be protected
 - Unless we want to make it public, which should be fast, easy, intuitive, and precise.
- today (end user tech)
  - Each device of the user probably has a different OS and UI
  - no inherent connection, only closed ecosystems

# The process model modern users deserve
- make a photo on my phone, it is immediately available on laptop
- install an app on my pc, it is also available on phone
- watch a video on the laptop, move it to phone with the same delay as moving it to another screen
- use phone touch input for game on pc
- ALL OF THIS: peer-to-peer, usually local

# WebAssembly to the rescue
- a lightweight portable runtime
- yes, it runs in the browser but it can run just like Java processes without a browser
- Improvements over Java Bytecode: fast startup (citation needed) + compilation from other languages
- My Vision:
  - User apps are all in Wasm, they don't care if they run in a browser, on iOS, or on Windows
  - New "Director" layer connects devices and processes running in them, in a private peer-to-peer network. (k11s for user apps)
  - local if possible (raspberry like box, could even be an old smartphone) otherwise using a provider of choice

# Challenges
- spin specific sdk, WASI is WIP
- multiple threads: crash (instead spawn many processes)
- network connection
- persistent storage




################
## October new take: An alternative to client-server / Is client-server architecture overused? / Why is everything client-server these days?

- Client-server
  - Server: Backend (centralized, database, authorization)
  - Client: Frontend (UI, local data and logic)
  - Communication: some API (e.g. HTTP calls)
  - Isolation: Both ends run on different OSs, one is controlled by the user the other by the service
  - Trust model: User must give all data to server, must trust the installed software to do nothing bad on local system, but at least the service doesn't need to trust the user in any way
- Emergence of new ways to think about applications is already happening.
  - Docker / Docker-Compose / Kubernetes is standard already on the backend
  - Local-first as a counter point to SaaS tries to bring multi-device collaboration without client-server architecture
  - WASM SaaS providers are popping up (TODO: so what?)
- New idea(s), not seen formulated exactly as such anywhere:
  - Generalize client-server with capability based nodes/threads (Caps "can display on a screen", "has camera driver", "can provide JWT", ...)
     - (maybe a bit like HarmonyOS?)
  - Every app has its own private network, by default no way to communicate to the outside world (but user still needs to trust the framework)
    - Instead of communication by IP, could be by Rust type system, but that's just a quirk of my implementation of the idea
  - WASM functions don't care where they are executed
  - System state is held coherent (could be replicated KV-store like CouchDB, could be central server with read/write locks)




################
## Reading notes

### Local First
https://www.local-first-cooperation.org/

1. Communicate Locally => Failure resistance
2. Build Autonomous Parts (collaboration with other edge devices must not be required for useful function, instead buffer messages etc)
3. Design Parts for Cooperation (Replication and conflict resolution, display things properly to users)
4. Accept Uncertainty when Making Decisions (confidently make decisions based on incomplete information) => failure/conflict resolution preferred over blocking
5. Foresee Dynamic Changes in the Network Neighbourhood (don't assume network topology, always react to changes, never assume some node will always be there)

=> Anti SaaS ?

#### How is my approach different?
- Coherence: The system provides globally consistent view, at a cost of availability (e.g. assume a central service is always available)
- (Not every node replicates everything)


### Actyx
https://github.com/Actyx/Actyx
A decentralized event database, streaming and processing engine that allows you to easily build local-first cooperative apps.
- durable event stream storage in peer-to-peer network using libp2p and ipfs-embed


### SPRAY
https://github.com/RAN3D/spray-wrtc
WebRTC peer-to-peer network by random sampling
Used for collaborative editor: https://web.archive.org/web/20200101022752/https://hal.archives-ouvertes.fr/hal-01303333/document
Also this paper: https://inria.hal.science/hal-01619906/document

<!-- links -->

[Actyx]: https://github.com/Actyx/Actyx
[Berkeley Sockets]: https://en.wikipedia.org/wiki/Berkeley_sockets
[component-model]: https://component-model.bytecodealliance.org/
[CRATE]: https://github.com/Chat-Wane/CRATE
[CRDT]: https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type
[infrared communication]: https://en.wikipedia.org/wiki/Infrared_Data_Association
[local-first]: https://www.local-first-cooperation.org/
[ncollide]: https://github.com/dimforge/ncollide
[rt-one-weekend]: https://raytracing.github.io/books/RayTracingInOneWeekend.html
[spray-paper]: https://inria.hal.science/hal-01619906/document
[wasm-pack]: https://github.com/rustwasm/wasm-pack
[WebPack]: https://github.com/webpack/webpack
[paddle]: https://github.com/jakmeier/paddle
[demo]: https://demos.jakobmeier.ch/distributed_wasm/
[webrtc]: https://webrtc.org/
[clumsy-rt-src]: https://github.com/jakmeier/distributed-wasm-rt-demo/tree/main/clumsy-rt
[spin-component]: https://github.com/jakmeier/distributed-wasm-rt-demo/tree/main/spin-component
[npu]: https://en.wikipedia.org/wiki/AI_accelerator
[barrelfish]: https://barrelfish.org/
[hotos09]: https://barrelfish.org/publications/barrelfish_hotos09.pdf
[osi]: https://en.wikipedia.org/wiki/OSI_model
[mmap]: https://www.man7.org/linux/man-pages/man2/mmap.2.html
[sosp09]: https://barrelfish.org/publications/barrelfish_sosp09.pdf
[tokio]: https://tokio.rs/
[posix]: https://en.wikipedia.org/wiki/POSIX
[win-api]: https://en.wikipedia.org/wiki/Windows_API
[future-rs]: https://doc.rust-lang.org/std/future/trait.Future.html
[wasm]: https://webassembly.org/
[wasm-abi]: https://component-model.bytecodealliance.org/design/canonical-abi.html
[shared-something]: https://github.com/yowl/wasm-component-model/blob/9c6863135145d0e815fa6cb6f3f249397c6ea748/design/mvp/FutureFeatures.md
[shared-nothing]: https://github.com/yowl/wasm-component-model/blob/9c6863135145d0e815fa6cb6f3f249397c6ea748/design/mvp/Explainer.md#component-invariants
[harmony-os]: https://www.harmonyos.com/en/