# Algorithmic Figgie Tournament - Testnet

This is my first time building out an exchange so guaranteed I was far away from best practices but I wanted to open source this so there's some examples online that people can view. Feel free to open PR's or send me your thoughts!

## Description

The testnet is a semi-mirror of the main exchange. It has the same model / structs as the main exchange but the logic of the game is a bit different. Instead of a static amount of cards being shuffled then dealt out, it's set up as a free-for-all so anyone can connect and test their WS & API connections at any time. For the game details, everyone will be dealt 3x cards of each suit and there's an unlimited amount of players that can play per game. To register, you'll need to send a POST request to register a temporary playerid then you can use that playerid to place orders throughout the game. At the end of the game the testent will wipe all the players so feel free to reconnect if you still need to test

## Docs

COMING SOON w/ a frontend website