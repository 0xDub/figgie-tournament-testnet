# Algorithmic Figgie Tournament - Testnet

This is my first time building out an exchange so guaranteed I was far away from best practices but I wanted to open source this so there's some examples online that people can view. Feel free to open PR's or send me your thoughts!

Splitting up the logic into modular functions is definitely something that should be done

## Description

The testnet is a semi-mirror of the main exchange. It has the same model / structs as the main exchange but the logic of the game is a bit different. Instead of a static amount of cards being shuffled then dealt out, it's set up as a free-for-all so anyone can connect and test their WS & API connections at any time. For the game details, everyone will be dealt 3x cards of each suit and there's an unlimited amount of players that can play per game. To register, you'll need to send a POST request to register a temporary playerid then you can use that playerid to place orders throughout the game. At the end of the game the testent will wipe all the players so feel free to reconnect if you still need to test

Note: player points are not true as the supply of cards isn't dynamic

## Infra Notes

I split up the high-level functions into 2 cores. The first core handles API requests, WS connections, and rate limit monitoring, whereas the second core handles the matching engine. In a prod setting I'd imagine that the server isols the matching engine core, allocates X cores to handling incoming and outgoing network traffic and shares data between them via some sort of lock-free buffer

For the outgoing traffic I'd imagine they send some sort of data to network out cores that then distribute this data in some fair way (tradfi goes the UDP multicast route for a reason). However in my testnet I let the matching engine send out the messages directly from the core. I admit this was part laziness but it works and there are only 4x players per real game so I don't imagine it'll be unfair to the point it gives anyone network edge

The serialization was quite interesting. I could have serialized the Option<>'s but this would have made it a pain for the client-side to handle so I opted to Stringify a lot of integers and if they're empty then send over an empty string. Doing this should make it easier to create parsing structs for the client-side

What's funny is that I actually ended up refactoring the exchange architecture 3x times to try and find some sort of best practice. Initially I didn't send back responses to order placement, just if the order was accepted or not. This didn't seem the most verbose to create a fulfilling dev environment tho so I switched it up and had the RestAPI send an order to the matching engine core with a freshly-created oneshot sender/receiver that it awaits. The matching engine itself then creates an HTTPResponse struct during the process_update() function and sends it back through the oneshot receiver which the RestAPI awaits then serializes and responds back with

## Docs

COMING SOON w/ a frontend website