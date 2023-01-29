---
layout: post
title: "The art of Byzantine benchmarking"
author: "Jakob Meier"
categories: Blogging
tags: [benchmarks,performance,systems,blockchain,rust,near]
image: 23/byzantine/carchase447.jpg
image_tooltip: "OpenGL ES benchmark, GFXBench Car Chase, frame 447"
# thumbnail_style: no-crop
lang: en
ref: byzantine-bench
techs:
  rust:
    title: "Rust"
    description: "Rust, my favorite programming language if I could only pick one."
    url: "https://rust-lang.org/"
    img: rust.png
  near:
    title: "NEAR Protocol"
    description: "A sharded layer 1 smart contract blockchain."
    url: "https://near.org/"
    img: near.svg
---

<p class="intro">
A traditional benchmark decides which of two implementations is superior in a controlled lab environment.
But what if you have a system with soft real-time constraints?
How can you proof they won't fail the real-time constraint?
And how wide is the margin?
Byzantine-Benchmarking is one possible approach to assess it.
</p>

I am in search of a performance-measuring framework for soft real-time systems.
Instead of looking at peak performance, or perhaps average or median performance, I want to evaluate the worst-case performance.
Something that gives me a guarantee that it will never, or at least rarely, run slower.

<details class="aside">
<summary>What is a soft real-time system?</summary>
<p>
Systems that have a deadline to get a workload done are called real-time.
We can broadly categorize them into soft and hard real-time systems, depending on how catastrophic missing the deadline is.
In a hard real-time system, failing to produce a result in time is a total system failure.
</p>

<p>
For example, rendering a 3D world in 60 frames per second requires a frame to be ready in 1000/60 ms ~= 16.6 ms.
Failing this requirement is inconvenient, as the display will stutter.
But it is not a complete system failure.
This is an example of a soft real-time requirement.
</p>

<p>
Hard real-time requirements exist in many physical devices.
To pick a particularly dramatic example, a helicopter that takes too long to compute the necessary change in forces will crash and catch fire.
A full system failure by any means.
</p>

<p>
To guarantee this will never happen, a helicopter will only use RTS hardware and software components.
If combined properly, an upper limit on computation time can be proven.
</p>
</details>

This requirement, to find the worst-case performance under all circumstances, reminds me a bit of
[Byzantine Fault Tolerance (BFT)](https://en.wikipedia.org/wiki/Byzantine_fault).
Therefore, I call the approach I am about to describe Byzantine benchmarking.

## Motivation

This type of work came up most recently in my work for the NEAR protocol blockchain.
This chain aims to produce a block once a second.
Missing the 1-second deadline will result in lower throughput for users and a smaller reward for validators.
This is a major service degradation for both parties but at least the system keeps on going, making it a typical soft real-time system.

The problem is, NEAR Protocol is built on top of components without any real-time guarantees.
Specialized real-time hardware and OSs exist and are quite common in embedded systems.
But our system runs on normal Linux machines at people's homes or on rented machines from cloud providers.

Most hardware is built to perform well on average.
But it can have large spikes of degraded performance.
This is problematic.

Say we measure the average performance and compare it to the 1s per block constraint.
Even if the average is 20% below the time constraint, it could be that 1 in 10 blocks takes more than 1s to produce.

Perhaps the maximum seems more appropriate than the average?
But the maximum of what really?
It is not that easy, as I will show in several steps below.

## Traditional Benchmark of ED25519 Signature Validation

Let's reduce the complexity to a single isolated component.
One component I had to benchmark was the verification of an ed25519 signature.
I will use it as a case study in this post.

To begin, I run a normal benchmark on the function, to establish a baseline.
The implementation we use is [verify](https://docs.rs/ed25519-dalek/1.0.1/src/ed25519_dalek/public.rs.html#329)
in the Rust library [ed25519-dalek](https://crates.io/crates/ed25519-dalek).

Keeping it simple, I used [cargo bench](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) in combination with [criterion](https://crates.io/crates/criterion) to evaluate the validation performance of random signatures.
It produces great statistical insights and reproducible results without much work on my end.
([Code is on Github](https://github.com/jakmeier/ed25519-benchmarks/blob/f87114f7e207754a1da0e8be0084e300b5a4201b/benches/criterion_benches.rs#L36-L49) if you are interested.)

But before running anything, I fixed my CPU's frequency.
(Expand the details below if you are curious why and how I did that.)

<details class="aside">
<summary>Fixing the CPU frequency</summary>
<p>
Modern CPUs dynamically increase voltage for heavy workloads, while staying at lower voltage points otherwise.
Higher voltage allows increasing the frequency.
Lower voltage reduces overall energy used, which translates to better thermals.
</p>

<p>
At the extreme, a single core is temporarily boosted to a high voltage and frequency, beyond what is thermally stable.
That is, after a short boost time, the CPU has to lower the voltage again, to avoid overheating.
</p>

<p>
A benchmark needs to be aware of this.
To measure sustained performance usually benchmarks run for long enough that several boosting cycles can occur.
Some systems will stabilize at one frequency, while others will periodically alternate between high and low frequencies.
</p>

<p>
But since we are ultimately after the worst-case performance, I want to ignore boosting.
I went into the BIOS and disabled frequency boosting.
</p>

<p>
But be careful, this is not enough!
The CPU can still scale in the thermally stable range.
I used
<code class="language-plaintext highlighter-rouge">cpupower frequency-set -g performance</code>
to set the frequency to a high but fixed number.
</p>
<p>
I could have also chosen the lowest allowed frequency.
But that would be too pessimistic since there is no way for a hypothetical byzantine actor to force such a low frequency of the CPU.
</p>
</details>

Straight from `cargo bench`, the graph below shows the median validation time at 53.26 μs with some outliers going up to 53.5 μs.

![Benchmark Distribution](/assets/img/23/byzantine/pdf_small.svg)

We could call it a day here and say that the worst-case performance is always below 53.5 μs.
But it's a trap!
Read on to understand why.


## Byzantine Benchmark of ED25519 Signature Validation

There are several problems with the benchmark above, namely:
1. `cargo bench` warms up the CPU to make results more consistent. This hides the hick-pus that real hardware has in practice.
2. We have no idea if the code has conditions under which it performs worse. (Think about what a garbage collector can do to worst-case performance.)
3. We used random sample inputs. In the byzantine model, we want the timing of the worst possible input.


Let's put the first issue aside for a moment.
I will come back to it soon.

For the second issue, I briefly looked through the code and checked if there are any caching, lazy initialization, or other features that can cause widely different performances among runs.
I found nothing.
And thanks to Rust being a systems programming language, I can be quite sure there are no hidden issues, like garbage collection.

For the third issue, I had to understand the code more thoroughly.
I need to understand, what would be the worst possible way to run the code.
Not just algorithmically but from a holistic system's engineering perspective.

Looking at the code, I quickly found the method `vartime_double_scalar_mul_basepoint`.
See the suspicious `vartime` prefix?
With that, the author tells me that the method takes a variable amount of time to execute.
I immediately know my performance evaluation must consider different inputs for this method.

<details class="aside">
<summary>Why mark a method as variable time?</summary>
<p>
When a crypto library operates on a secret (private key) it is often required that the timing of the computation does not reveal anything about the secret.
</p>
<p>
A bad example would be a password string comparison that compares character after character and short-circuits as soon as one character mismatches.
This could be exploited if an attacker can guess passwords and measure the time accurately.
They could try out passwords with different starting characters and compare which takes the longest to be rejected.
A guess that starts with the correct character will take slightly longer than other guesses.
</p>
<p>
Like this, assuming a perfect time measurement, the attacker can learn the first character with a relatively small number of guesses.
With an imperfect time measurement, statistics and many repetitions can often achieve the same goal.
</p>
<p>
Then they may repeat this procedure to guess the next character.
Until they have guessed the full password.
</p>

<p>
The described strategy is a time-based example of a side-channel attack.
It works because a side channel (timing) leaks more information about the password than just a simple yes or no in the password checker response (the main channel).
</p>
<p>
To counteract this, a safe password checker will always take a constant amount of time.
For example, the implementation could always check all characters, instead of short-circuiting.
</p>
<p>
In the case of the signature validation, ed25519_dalek uses variable time multiplication because it is more efficient than a constant time implementation.
This is not a problem because a signature validation only operates on public keys, not on private keys.
</p>
</details>


Quick profiling shows me 91.5% of the total time is spent on this method. 
So it is not negligible for the total performance and it seems reasonable to dig deeper.

![Flamegraph showing over 90% of samples in the mentioned function.](/assets/img/23/byzantine/flamegraph.svg)
*Flamegraph of the verify method.*

### Byzantine Input

So my goal was clear: Find the input that produces the slowest execution time for the `vartime_double_scalar_mul_basepoint` method.
I started by examining the code.

```rust
/// Compute \\(aA + bB\\) in variable time, where \\(B\\) is the Ed25519 basepoint.
pub fn mul(a: &Scalar, A: &EdwardsPoint, b: &Scalar) -> EdwardsPoint {
    let a_naf = a.non_adjacent_form(5);
    let b_naf = b.non_adjacent_form(8);

    // Find starting index
    let mut i: usize = 255;
    for j in (0..256).rev() {
        i = j;
        if a_naf[i] != 0 || b_naf[i] != 0 {
            break;
        }
    }

    let table_A = NafLookupTable5::<ProjectiveNielsPoint>::from(A);
    let table_B = &constants::AFFINE_ODD_MULTIPLES_OF_BASEPOINT;

    let mut r = ProjectivePoint::identity();
    loop {
        let mut t = r.double();

        if a_naf[i] > 0 {
            t = &t.to_extended() + &table_A.select(a_naf[i] as usize);
        } else if a_naf[i] < 0 {
            t = &t.to_extended() - &table_A.select(-a_naf[i] as usize);
        }

        if b_naf[i] > 0 {
            t = &t.to_extended() + &table_B.select(b_naf[i] as usize);
        } else if b_naf[i] < 0 {
            t = &t.to_extended() - &table_B.select(-b_naf[i] as usize);
        }

        r = t.to_projective();

        if i == 0 {
            break;
        }
        i -= 1;
    }

    r.to_extended()
}
```

Zooming in, the variable time comes from this section:

```rust
if a_naf[i] > 0 {
    t = &t.to_extended() + &table_A.select(a_naf[i] as usize);
} else if a_naf[i] < 0 {
    t = &t.to_extended() - &table_A.select(-a_naf[i] as usize);
}

if b_naf[i] > 0 {
    t = &t.to_extended() + &table_B.select(b_naf[i] as usize);
} else if b_naf[i] < 0 {
    t = &t.to_extended() - &table_B.select(-b_naf[i] as usize);
}
```

Without going into cryptographic details too much, `a_naf` and `b_naf` are 256-bit scalar numbers.
They are put into the [non-adjacent form](https://en.wikipedia.org/wiki/Non-adjacent_form) which is then represented as a 256-byte array. 
Important for us, this representation is guaranteed to have many zero bytes.
This makes multiplication efficient.
Logically, I need to find an input with a few zero bytes.

Now, the problem is these numbers are computed from the input following the steps defined in the [ed25519 signature scheme](https://www.rfc-editor.org/rfc/rfc8032).
One of the steps involves a SHA-512 hash, which makes reversing this impossible.
I think it might be possible to reverse everything aside from the hash and use it to measure the performance of just the elliptic curve computations, without the hashing performance.
But I would rather have a full and valid signature for my test input.

Making my life easy, I decided to generate a large number of random keys and simply pick those with the most multiplication steps.
For that, I forked the original code and have it print the number of zeroes.

Most random inputs have around 71 to 72 non-zero values in `a_naf` and `b_naf` combined.
Generating inputs for about an hour, I could not find any with 80 or more non-zero values.
But I could quickly find some with 79 non-zeroes.
I selected 10 of those, let's call them Byzantine samples.
Then I compared the verification performance to 10 random inputs.

![Violin plot comparing random to byzantine samples](/assets/img/23/byzantine/criterion_sampling.svg)

Success!
The Byzantine samples on the right are slower to validate than the random inputs on the left.
Not by much, the slowest 90th percentile was just below 56 μs.
Not a huge change to the original measurement but useful information nevertheless.

Better yet, the Byzantine inputs are statistically more stable, especially on the lower end.
Statistical stability is always good for a benchmark!

Overall, I am quite happy with the Byzantine input sample, which will be more relevant to evaluate than a random input.

![A graphically expensive image to render, showing a tank in Manhattan.](/assets/img/23/byzantine/manhattan30131.jpg)
*GFXBench30 frame 131, a classic input for mobile GPU benchmarking because of how hard it is to render.*

### No Warming Up With Byzantines

Next, the warm-up problem.
In traditional benchmarking, warming up caches for a couple of iterations before starting the measurements is good practice.
It makes the results more reproducible and reduces variance.

But I want to know the worst-case performance.
Playing around with criterion's settings, I tried warm-up = 1 ns (0 is not allowed) and I set the number of iterations to 10 (again, lower is not allowed).

This way, I should get the initial performance, without warmed-up caches or primed branch predictors.
The results are below.

![Violin plot comparing warmed-up benchmark to non-warmed-up ones](/assets/img/23/byzantine/criterion_sampling_no_warmup.svg)

On the right, the results without warming up.
On the left, the previous results for comparison.
Not much has changed, what is going on?

Looking at the individual 10 measurements of one sample, I see the problem.
The first iteration was slower but it warmed up quickly after that.

![Plot that shows a curve of increasingly fast samples](/assets/img/23/byzantine/criterion_sampling_no_warmup_single_sample.svg)

I decided to move on from criterion and write a custom benchmarking loop.
In this loop, CPU caches are invalidated before each iteration.

<details class="aside">
<summary>Invalidating CPU caches</summary>
<p>
A modern CPU has several layers of caches, L1/L2/L3, which are fast-access memory that stores the recently used data.
It makes subsequent access to the same data faster.
</p>

<p>
On some layers, there are separate data and instruction caches.
The first holds non-executable data, the latter binary code for execution.
</p>

<p>
One might want to deactivate caches entirely for benchmarks.
But that would measure unrealistic performance characteristics since every layer of a modern system is optimized for enabled CPU caches.
</p>

<p>
Instead, I want to start with empty caches and then let the  benchmarked implementation use them as usual. 
</p>

<p>
I evaluated two ways of invalidating caches.
First, the special x86 instruction `WBINV` (write-back and invalidate) using a <a href="https://github.com/batmac/wbinvd">small kernel module</a>. It commands the CPU to clear all caches.
</p>

<p>
Second, read enough random data before starting each iteration of the benchmark.
This only affects data caches and is much slower than the first approach.
But it is portable and gives statistically indistinguishable results, compared to the WBINV approach.
</p>
</details>

With that, I managed to get a slow performance consistently.

![Violin plot comparing cold and warm caches](/assets/img/23/byzantine/custom_bench.svg)

Now, look at that!
The median at around 60 μs, with the 90th percentile going up to 62.5 μs.
This is 9 μs or, 16.8% higher than the original guess.
That's good enough for today.

To benchmark the entire system, I would have to do the same kind of analysis for each component of the system.
This is quite time-consuming.
And not all components are quite as easy to analyze as ed25519 verification.
But worth the time if bad timing kills the system!

There is a second blog post in my head, actually some parts already in writing, that deals with databases and disk IO in Byzantine benchmarking.
But this topic is hard.
I don't think I have quite cracked it, yet.
Stay tuned to read more about that later, just don't expect perfect answers too soon.

## Closing words

Benchmarks are often used to compare average performance for a given, usually intensive workload.
Sometimes, the reported results include the 99th percentile or even the maximum measurement.
But the absolute worst case, for all possible inputs, cannot be determined by traditional benchmarks.

Evaluating the truly worst case can be difficult.
The common black-box approach that averages over many random inputs is no good here.
A bit of glancing into the implementation and a careful benchmark setup is required.

The good news is, the described byzantine benchmarking approach does not compromise on reproducibility or statistical stability.
And the ideas presented here apply easily to other systems.

Maybe next time measuring the performance of a system, think twice.
Do you care about the average performance of the system when it executes the same type of work in a tight loop?
Or do you care more about the worst performance a user will experience?

## Links

Benchmarking and plotting: [github.com/jakmeier/ed25519-benchmarks](https://github.com/jakmeier/ed25519-benchmarks)

Illustrative GFXBench frames: [gfxbench.com](https://gfxbench.com/benchmark.jsp)

<!-- Discussion and comments: TODO -->
