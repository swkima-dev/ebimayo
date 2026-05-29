# ebimayo coding standards

This is a short document describing the preferred coding style for this "ebimayo" project.
Coding style is very personal, and it may not be desirable
to impose strict coding standards on personal development projects.
However, the project aims to improve developer capabilities, especially in writing maintainable code and raising awareness of production-level quality.
Therefore, participants in this project (i.e. me) are required to follow this rule while coding.

# major principles

The following one principle is the most important in this project. All rules are governed by the principle, and no rule can override the principle.

The purpose of these principles is not ideological, but educational and technical.

## PRINCIPLE) Code should be easy to understand

In other words, "code should be written to minimize the time it would take for someone else to understand it.", (Dustin et al. 2012, p. 3).

Every single code must be written with the idea that it will be easy for others, or your future yourself, to understand.

# Rules

## 1) Formatting

USE `rustfmt`. That's it.

## 2) Lint

USE Clippy. That's it.


## 3) Do not repeat yourself (DRY Principle)

Do not copy and paste code.

Copying and pasting code leads to duplicated logic, which increases the cost of future modifications, as the same change must be applied in multiple places.
If the same code appears more than once, it should be extracted, consolidated into a single implementation, and invoked from other locations.

In practice, consolidating duplicated code is often performed during refactoring
(i.e., improving the internal structure of software without changing its external behavior).
However, duplication should be avoided as much as possible from the initial implementation stage.

## 4) Single responsibility

Methods and functions should have a clear and single responsibility.

When a method or function has only one responsibility, it becomes easier to write unit tests, thereby improving the testability of the code.
In addition, having a single responsibility makes it easier to identify modification points when changes are required, which reduces the time needed for bug fixes and maintenance.

(You may recognize this concept as the "S" in the SOLID principles: Single Responsibility Principle.)

For example, suppose there is a method named `checkAndDo()` that performs both validation and execution.
In this case, the method should be split into two separate methods: `check()` and `do()`.

If an error exists in the validation logic of `checkAndDo()`, understanding the behavior of `do()` also becomes necessary.
By separating responsibilities, changes can be made to `check()` alone without affecting `do()`.

This principle applies not only to methods and functions, but also to class design.

## 5) Breaking long lines and strings

It's easy to imagine that if a line is too long, readability will decrease.

`rustfmt` automatically breaks lines when the number of characters per line exceeds 100.
Of course, you can rely on this, but in other programming languages, the recommended limit is usually 80 characters per line.
(The Linux Kernel's coding style says, "The preferred limit on the length of a single line is 80 columns.")
The important thing here is not whether it's 80 characters or 100 characters, but that you should be careful not to rely too much on rustfmt and write code that's too long.

## 6) Minimize nesting
Deep nesting takes up a lot of human working memory and is difficult to understand.

For example, this sample code forces you to repeatedly look back and double-check which block conditions you are currently in:

```c
if (user_result == SUCCESS) {
 if (permission_result != SUCCESS) {
 reply.WriteErrors("error reading permissions");
 reply.Done();
 return;
 }
 reply.WriteErrors("");
} else {
 reply.WriteErrors(user_result);
}
reply.Done();
```

If you simply insert a `permission_result` branch after implementing the `user_result` branching process, you will end up with code like this.
These type of codes can be improved by returning early:

```c
if (user_result != SUCCESS) {
 reply.WriteErrors(user_result);
 reply.Done();
 return;
}
if (permission_result != SUCCESS) {
 reply.WriteErrors(permission_result);
 reply.Done();
 return;
}
reply.WriteErrors("");
reply.Done();
```

This code only has one level of nesting, instead of two. It's Intuitive and easy to read.

## 7) Naming

Basically, think of a name as a short comment made up of "specific words", and you should not use names that are misleading. 
For example, you should only use the name "tmp" for short-lived, temporary variables that are of the utmost importance. (= Don't be lazy and name it `tmp`)

Encoding the type of a function into the name (so-called Hungarian notation) is not recommended.
If naming is solid, Hungarian notation is unnecessary, and it increases the cost of modifying code.

Rust has an official style guide for naming, which should also be followed.
(Quoting from *Rust Style Guide*↓)

- Types shall be `UpperCamelCase`  
- Enum variants shall be `UpperCamelCase`  
- Struct fields shall be `snake_case`  
- Function and method names shall be `snake_case`  
- Local variables shall be `snake_case`  
- Macro names shall be `snake_case`  
- Constants (`const`s and immutable `static`s) shall be `SCREAMING_SNAKE_CASE`  
- When a name is forbidden because it is a reserved word (such as `crate`), either use a raw identifier (e.g., `r#crate`) or use a trailing underscore (e.g., `crate_`). Don’t misspell the word (e.g., avoid `krate`)

## 8) Commenting

"Comments are good, but there is also a danger of over-commenting. 
NEVER try to explain HOW your code works in a comment: it’s much better to write the code so that the working is obvious, and it’s a waste of time to explain badly written code."
(The Linux Kernel Coding style)

Basically, Comments should be up to date and have a high information-to-space ratio.
To clarify, here's what I mean by information density:(Dustin et al. 2012, p. 64)

```
// This class acts as a caching layer to the database.
```

You should not write it like this:(Dustin et al. 2012, p. 64)

```
// This class contains a number of members that store the same information as in the
// database, but are stored here for speed. When this class is read from later, those
// members are checked first to see if they exist, and if so are returned; otherwise the
// database is read from and that data stored in those fields for next time.
```

Especially in Rust programinng, prefer to put a comment on its own line.
Where a comment follows code, there should be a single space before it. 
Where a block comment is inline, there should be surrounding whitespace as if it were an identifier or keyword. 
There should be no trailing whitespace after a comment or at the end of any line in a multi-line comment. Examples:

```c
// A comment on an item.
struct Foo { ... }

fn foo() {} // A comment after an item.

pub fn foo(/* a comment before an argument */ x: T) {...}
```
Comments should usually be complete sentences.
Start with a capital letter, end with a period (.). An inline block comment may be treated as a note without punctuation.

## 9) Error handling

ebimayo folows the usual Rust error handling conventions:

  https://doc.rust-lang.org/book/ch09-00-error-handling.html

Please note that panicking should be very rare and used only with a good reason. In almost all cases, a fallible approach should be used, typically returning a Result.

This project aims for production-level quality, so basically you should use `expect` rather than unwrap and give more context about why the operation is expected to always succeed.

# References

- *The Art of Readable Code* by Dustin Boswell and Trevor
Foucher. Copyright 2012 Dustin Boswell and Trevor Foucher, 978-0-596-80229-5.
- The Linux Kernel - coding style, https://www.kernel.org/doc/html/latest/process/coding-style.html
- The Linux Kernel - Rust Coding Guidelines, https://docs.kernel.org/rust/coding-guidelines.html
- Rust Style Guide, https://doc.rust-lang.org/style-guide/index.html
- The Rust Programming Language, https://doc.rust-lang.org/book/title-page.html
- Future Enterprise Coding Standards - Javaコーディング規約, https://future-architect.github.io/coding-standards/documents/forJava/Java%E3%82%B3%E3%83%BC%E3%83%87%E3%82%A3%E3%83%B3%E3%82%B0%E8%A6%8F%E7%B4%84.html
