# RUST PROJECT PROPOSAL
## By: Šimon Mizerák (xmizeraks) & Jakub Petrík (xpetrikj)

### Introduction
For this course project we have decided to develop a Password Manager in the Rust language. This project will be a UI-based desktop application capable of securely encrypting, storing and retrieving a user’s password based on Argon2. The user will be able to securely store or generate passwords for their accounts, making their online experience smoother and more accessible.
Throughout the development and research we wish to learn more about encryption and data security. Rust is a suitable language for both of these concepts, due to its secure nature and we hope to use the language to its fullest during development.
The main problems we will be trying to solve during our time working on this project will be securely storing the user’s passwords and rotating the keys for them.

### Requirements
+ Custom encryption model based on Argon2
+ Securely storing the master key and salt used
+ Secure database, which will hold all the passwords
+ Generator able to make safe and secure passwords
+ Log-in system using Multi Factor Verification
+ Periodic Key Rotation system
+ User friendly UI, which will display their passwords

### Dependencies
+ argon2 - secure key derivation
+ aes-gsm - used for encryption
+ rusqlite - creating a database for passwords 
+ keyring - secure OS storing of master key
+ tui - used for user interface
