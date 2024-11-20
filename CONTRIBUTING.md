<!-- omit in toc -->
# Contributing to Movement

First off, thanks for taking the time to contribute! ❤️

All types of contributions are encouraged and valued. See the [Table of Contents](#table-of-contents) for different ways to help and details about how this project handles them. Please make sure to read the relevant section before making your contribution. It will make it a lot easier for us maintainers and smooth out the experience for all involved. The community looks forward to your contributions. 🎉

> And if you like the project, but just don't have time to contribute, that's fine. There are other easy ways to support the project and show your appreciation, which we would also be very happy about:
> - Star the project
> - Tweet about it
> - Refer this project in your project's readme
> - Mention the project at local meetups and tell your friends/colleagues

<!-- omit in toc -->
## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [I Have a Question](#i-have-a-question)
  - [I Want To Contribute](#i-want-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Enhancements](#suggesting-enhancements)
  - [Your First Code Contribution](#your-first-code-contribution)
  - [Improving The Documentation](#improving-the-documentation)
- [Styleguides](#styleguides)
  - [Commit Messages](#commit-messages)
- [Join The Project Team](#join-the-project-team)


## Code of Conduct

This project and everyone participating in it is governed by the
[Movement Code of Conduct](https://github.com/movementlabsxyz/movement/blob/main/CODE_OF_CONDUCT.md).
By participating, you are expected to uphold this code. Please report unacceptable behavior
to <hello@movementlabs.xyz>.


## I Have a Question

> If you want to ask a question, we assume that you have read the available [Documentation](https://docs.movementnetwork.xyz/).

Before you ask a question, it is best to search for existing [Issues](https://github.com/movementlabsxyz/movement/issues) that might help you. In case you have found a suitable issue and still need clarification, you can write your question in this issue. It is also advisable to search the internet for answers first.

If you then still feel the need to ask a question and need clarification, we recommend the following:

- Open an [Issue](https://github.com/movementlabsxyz/movement/issues/new/choose).
- Provide as much context as you can about what you're running into.
- Provide project and platform versions (nodejs, npm, etc), depending on what seems relevant.

We will then take care of the issue as soon as possible.

<!--
You might want to create a separate issue tag for questions and include it in this description. People should then tag their issues accordingly.

Depending on how large the project is, you may want to outsource the questioning, e.g. to Stack Overflow or Gitter. You may add additional contact and information possibilities:
- IRC
- Slack
- Gitter
- Stack Overflow tag
- Blog
- FAQ
- Roadmap
- E-Mail List
- Forum
-->

## I Want To Contribute

> ### Legal Notice <!-- omit in toc -->
> When contributing to this project, you must agree that you have authored 100% of the content, that you have the necessary rights to the content and that the content you contribute may be provided under the project licence.

### Reporting Bugs

<!-- omit in toc -->
#### Before Submitting a Bug Report

A good bug report shouldn't leave others needing to chase you up for more information. Therefore, we ask you to investigate carefully, collect information and describe the issue in detail in your report. Please complete the following steps in advance to help us fix any potential bug as fast as possible.

- Make sure that you are using the latest version.
- Determine if your bug is really a bug and not an error on your side e.g. using incompatible environment components/versions (Make sure that you have read the [documentation](https://docs.movementnetwork.xyz/). If you are looking for support, you might want to check [this section](#i-have-a-question)).
- To see if other users have experienced (and potentially already solved) the same issue you are having, check if there is not already a bug report existing for your bug or error in the [bug tracker](https://github.com/movementlabsxyz/movement/issues?q=label%3Abug).
- Also make sure to search the internet (including Stack Overflow) to see if users outside of the GitHub community have discussed the issue.
- Collect information about the bug:
  - Stack trace (Traceback)
  - OS, Platform and Version (Windows, Linux, macOS, x86, ARM)
  - Version of the interpreter, compiler, SDK, runtime environment, package manager, depending on what seems relevant.
  - Possibly your input and the output
  - Can you reliably reproduce the issue? And can you also reproduce it with older versions?

<!-- omit in toc -->
#### How Do I Submit a Good Bug Report?

> You must never report security related issues, vulnerabilities or bugs including sensitive information to the issue tracker, or elsewhere in public. Instead sensitive bugs must be sent by email to <hello@movementlabs.xyz>.
<!-- You may add a PGP key to allow the messages to be sent encrypted as well. -->

We use GitHub issues to track bugs and errors. If you run into an issue with the project:

- Open an [Issue](https://github.com/movementlabsxyz/movement/issues/new?template=bug_report.md). (Since we can't be sure at this point whether it is a bug or not, we ask you not to talk about a bug yet and not to label the issue.)
- Explain the behavior you would expect and the actual behavior.
- Please provide as much context as possible and describe the *reproduction steps* that someone else can follow to recreate the issue on their own. This usually includes your code. For good bug reports you should isolate the problem and create a reduced test case.
- Provide the information you collected in the previous section.

Once it's filed:

- The project team will label the issue accordingly.
- A team member will try to reproduce the issue with your provided steps. If there are no reproduction steps or no obvious way to reproduce the issue, the team will ask you for those steps and mark the issue as `needs-repro`. Bugs with the `needs-repro` tag will not be addressed until they are reproduced.
- If the team is able to reproduce the issue, it will be marked `needs-fix`, as well as possibly other tags (such as `critical`), and the issue will be left to be [implemented by someone](#your-first-code-contribution).

<!-- You might want to create an issue template for bugs and errors that can be used as a guide and that defines the structure of the information to be included. If you do so, reference it here in the description. -->


### Suggesting Enhancements

This section guides you through submitting an enhancement suggestion for Movement, **including completely new features and minor improvements to existing functionality**. Following these guidelines will help maintainers and the community to understand your suggestion and find related suggestions.

<!-- omit in toc -->
#### Before Submitting an Enhancement

- Make sure that you are using the latest version.
- Read the [documentation](https://docs.movementnetwork.xyz/) carefully and find out if the functionality is already covered, maybe by an individual configuration.
- Perform a [search](https://github.com/movementlabsxyz/movement/issues) to see if the enhancement has already been suggested. If it has, add a comment to the existing issue instead of opening a new one.
- Find out whether your idea fits with the scope and aims of the project. It's up to you to make a strong case to convince the project's developers of the merits of this feature. Keep in mind that we want features that will be useful to the majority of our users and not just a small subset. If you're just targeting a minority of users, consider writing an add-on/plugin library.

<!-- omit in toc -->
#### How Do I Submit a Good Enhancement Suggestion?

Enhancement suggestions are tracked as [GitHub issues](https://github.com/movementlabsxyz/movement/issues). You can create a new enhancement issue via this [Template](https://github.com/movementlabsxyz/movement/issues/new?template=feature_request.md)

- Use a **clear and descriptive title** for the issue to identify the suggestion.
- Provide a **step-by-step description of the suggested enhancement** in as many details as possible.
- **Describe the current behavior** and **explain which behavior you expected to see instead** and why. At this point you can also tell which alternatives do not work for you.
- You may want to **include screenshots or screen recordings** which help you demonstrate the steps or point out the part which the suggestion is related to. You can use [LICEcap](https://www.cockos.com/licecap/) to record GIFs on macOS and Windows, and the built-in [screen recorder in GNOME](https://help.gnome.org/users/gnome-help/stable/screen-shot-record.html.en) or [SimpleScreenRecorder](https://github.com/MaartenBaert/ssr) on Linux. <!-- this should only be included if the project has a GUI -->
- **Explain why this enhancement would be useful** to most Movement users. You may also want to point out the other projects that solved it better and which could serve as inspiration.

<!-- You might want to create an issue template for enhancement suggestions that can be used as a guide and that defines the structure of the information to be included. If you do so, reference it here in the description. -->

### Your First Code Contribution
<!-- TODO
include Setup of env, IDE and typical getting started instructions?

-->
If you’re new to contributing to open source or unsure where to start, we’ve got you covered. We recommend the following steps for your first code contribution:

- **Find a “good first issue”** – These issues are marked to help new contributors start with smaller, manageable contributions.
- **Set up the project** – Follow the instructions in the repository’s [README](https://github.com/movementlabsxyz/movement/blob/main/README.md) to set up the project locally.
- **Get in touch** – Comment on the issue you’re interested in working on. Our maintainers will assist if you need any guidance.
- **Make the changes** – Start small; fix a bug, update documentation, or work on a minor feature.
- **Submit a Pull Request (PR)** – Once you’re ready, open a PR with a clear description of your changes.

We’ll review your code, offer feedback, and help with any questions.

### Improving The Documentation
<!-- TODO
Updating, improving and correcting the documentation

-->
Documentation improvements are always appreciated and highly valuable to the project. Here are some ways to improve documentation:

- **Correcting typos or grammar** – Small changes can make a big difference in readability.
- **Clarifying confusing sections** – If anything seems unclear to you, it likely will to others as well.
- **Adding examples** – Code examples are incredibly helpful in showing how things work in practice.
- **Updating outdated information** – Ensure everything reflects the current state of the project.

When making documentation updates, remember to follow the **Styleguides** below.

### Styleguides

We build with Rust.  Please follow [Rust Style Guide](https://doc.rust-lang.org/style-guide/index.html) for further information.

### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification for structuring commit messages, and **signed commits** are required. This helps in keeping a clear, structured, and verified commit history.

#### Structure

A Conventional Commit message consists of three parts: **type**, **scope** (optional), and **description**. The format is as follows:

```
<type>(<scope>): <description>
```

- **Type**: Describes the purpose of the commit. Examples: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.
- **Scope**: Optional, specifies the part of the codebase the commit affects (e.g., `api`, `docs`, `cli`).
- **Description**: A brief summary of the changes, written in imperative mood (e.g., "Add new feature").

Examples:
- `feat(api): add new authentication method`
- `fix(parser): handle null values correctly`
- `docs: update contributing guide`

#### Signing Commits

To sign a commit, use:

```bash
git commit -S -m "feat(api): add new authentication method"
```

This ensures that each commit is verified and follows the project’s security standards.

This style guide should help contributors maintain a consistent and idiomatic Rust codebase!  

## Join The Project Team
<!-- TODO -->
We’re always looking for passionate contributors to join the project team! Team members:

- Review PRs and issues
- Propose new features
- Help maintain the project’s quality and direction
- If you're interested, please let us know in an issue, and we’ll be happy to discuss how you can get involved further!

<!-- omit in toc -->
## Attribution
This guide is based on the **contributing-gen**. [Make your own](https://github.com/bttger/contributing-gen)!
