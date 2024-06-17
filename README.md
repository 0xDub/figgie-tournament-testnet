# Algorithmic Figgie Tournament - Testnet

This was my first time building out an exchange so I was probably far from best practices but I wanted to open source this so there's some examples online that people can view. My hope is that it can inspire some people or get the gears turning on possible architectures that are used in prod. Let the creativity flow for network and architectural alpha!

## Description

This testnet is a near identical mirror of the main exchange. It has the same model / structs as the main exchange tho the logic of the game is a bit different as it's solely geared for testing connections / requests. To register, send an empty `POST` request to `/register_testnet` with a chosen `playerid` in the request's header. You'll be given back a random name that you can use to track your actions on the subsequent updates that you receive from the websocket connection

To prune connections the temporary players & their player_id's are cleared after every game (4x rounds, 2min each), but feel free to continue on after sending another `POST` request to `/register_testnet`

Note: player points are not true as the supply of cards aren't dynamic (everyone gets 3x of each suit)

## Infra Notes (for devs)

Building this out was quite fun and I ran into a lot of interesting design questions. I listed some thoughts below if you'd like to read them

So to start, I decided to split up the high-level functions into their own distinct cores. The first core handles Incoming Websocket Connections, serving RestAPI requests, and monitors player rate limits. The second core solely processes updates for the matching engine then sends out the updates through the websockets (that are shared via a player_name -> connection map)

I figured this was a good step in the right direction for best practices as I've heard that exchange's tend to favor low standard deviation of latency + fairness, opposed to pure raw processing speed. There are (at least) a few problems with the current infra though, one of them you might have caught onto in the last paragraph above. The hotpath core is sending out network IO instead of offloading that onto a dedicated core. This'll hurt the cache of the hotpath core and also cause network interrupts when we could be juicing out a lot more speed in the hotpath core

In a prod setting I'd imagine that they split up the cores, isolcpu the computationally-centric cores, and share data via some busy-spun lock-free buffer. This all gets quite interesting tho when thinking about state machine tech + multiple location / AZ redundancy features that many exchanges likely implement. I'd love to hear how people have tackled this issue before!

For serialization, my main thought here was to try and make it as easy as possible for the client-side to parse the response. I'm not sure about y'all but I love when data is easy to parse (standardization helps!). Some crypto exchange's have done a pretty good job at this so the response takes after them via lists of price levels

Anyways, this was a really fun mini infra rabbit hole to go down! Hats off to all the exchange devs out there, this stuff can get quite challenging

## Docs

COMING SOON w/ a frontend website