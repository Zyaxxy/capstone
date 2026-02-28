import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Capstone } from "../target/types/capstone";
import { expect } from "chai";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createMint,
  mintTo,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID
} from "@solana/spl-token";

describe("capstone-auction", () => {
  const provider = anchor.AnchorProvider.env()!;
  anchor.setProvider(provider);

  const program = anchor.workspace.capstone as Program<Capstone>;

  const maker = provider.wallet;
  const bidder1 = anchor.web3.Keypair.generate();
  const bidder2 = anchor.web3.Keypair.generate();
  const crank = anchor.web3.Keypair.generate();

  let nftMint: anchor.web3.PublicKey;
  let bidMint: anchor.web3.PublicKey;
  let makerNftAta: anchor.web3.PublicKey;
  let makerBidAta: anchor.web3.PublicKey;
  let bidder1BidAta: anchor.web3.PublicKey;
  let bidder2BidAta: anchor.web3.PublicKey;
  let winnerNftAta: anchor.web3.PublicKey;

  const seed1 = new anchor.BN(Math.floor(Math.random() * 1001));
  const seed2 = new anchor.BN(Math.floor(Math.random() * 1002));
  let auctionPda: anchor.web3.PublicKey;
  let vaultNft: anchor.web3.PublicKey;
  let vaultBid: anchor.web3.PublicKey;
  let bidRecord1: anchor.web3.PublicKey;
  let bidRecord2: anchor.web3.PublicKey;

  let endTime: number;

  before(async () => {
    console.log("Funding test accounts from main provider wallet...");
    const transferTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: maker.publicKey,
        toPubkey: bidder1.publicKey,
        lamports: 0.1 * anchor.web3.LAMPORTS_PER_SOL,
      }),
      anchor.web3.SystemProgram.transfer({
        fromPubkey: maker.publicKey,
        toPubkey: bidder2.publicKey,
        lamports: 0.1 * anchor.web3.LAMPORTS_PER_SOL,
      }),
      anchor.web3.SystemProgram.transfer({
        fromPubkey: maker.publicKey,
        toPubkey: crank.publicKey,
        lamports: 0.1 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(transferTx);

    nftMint = await createMint(provider.connection, maker.payer, maker.publicKey, null, 0);
    bidMint = await createMint(provider.connection, maker.payer, maker.publicKey, null, 6);

    makerNftAta = getAssociatedTokenAddressSync(nftMint, maker.publicKey);
    makerBidAta = getAssociatedTokenAddressSync(bidMint, maker.publicKey);

    let tx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(maker.publicKey, makerNftAta, maker.publicKey, nftMint)
    );
    await provider.sendAndConfirm(tx);
    await mintTo(provider.connection, maker.payer, nftMint, makerNftAta, maker.publicKey, 1);

    bidder1BidAta = getAssociatedTokenAddressSync(bidMint, bidder1.publicKey);
    bidder2BidAta = getAssociatedTokenAddressSync(bidMint, bidder2.publicKey);

    tx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(maker.publicKey, bidder1BidAta, bidder1.publicKey, bidMint),
      createAssociatedTokenAccountInstruction(maker.publicKey, bidder2BidAta, bidder2.publicKey, bidMint)
    );
    await provider.sendAndConfirm(tx);

    await mintTo(provider.connection, maker.payer, bidMint, bidder1BidAta, maker.publicKey, 1000_000_000);
    await mintTo(provider.connection, maker.payer, bidMint, bidder2BidAta, maker.publicKey, 1000_000_000);
  });

  it("Makes an auction", async () => {
    endTime = Math.floor(Date.now() / 1000) + 10;

    [auctionPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("auction"), maker.publicKey.toBuffer(), seed1.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    vaultNft = getAssociatedTokenAddressSync(nftMint, auctionPda, true);
    vaultBid = getAssociatedTokenAddressSync(bidMint, auctionPda, true);

    await program.methods
      .makeAuction(seed1, new anchor.BN(endTime), new anchor.BN(1))
      .accountsStrict({
        maker: maker.publicKey,
        nftMint: nftMint,
        bidMint: bidMint,
        makerNftAta: makerNftAta,
        auction: auctionPda,
        vaultNft: vaultNft,
        vaultBid: vaultBid,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultNftBalance = (await provider.connection.getTokenAccountBalance(vaultNft)).value.uiAmount;
    expect(vaultNftBalance).to.equal(1);
  });

  it("Handles multiple bids and raises", async () => {
    [bidRecord1] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bids"), auctionPda.toBuffer(), bidder1.publicKey.toBuffer()],
      program.programId
    );

    [bidRecord2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bids"), auctionPda.toBuffer(), bidder2.publicKey.toBuffer()],
      program.programId
    );

    // Bidder 1 bids 100
    await program.methods.bid(new anchor.BN(100_000_000))
      .accountsStrict({
        bidder: bidder1.publicKey,
        auction: auctionPda,
        bidRecord: bidRecord1,
        bidderBidAta: bidder1BidAta,
        vaultBid: vaultBid,
        bidMint: bidMint,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bidder1])
      .rpc();

    // Bidder 2 bids 200
    await program.methods.bid(new anchor.BN(200_000_000))
      .accountsStrict({
        bidder: bidder2.publicKey,
        auction: auctionPda,
        bidRecord: bidRecord2,
        bidderBidAta: bidder2BidAta,
        vaultBid: vaultBid,
        bidMint: bidMint,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bidder2])
      .rpc();

    // Bidder 1 raises by 150 (Total 250)
    await program.methods.bid(new anchor.BN(150_000_000))
      .accountsStrict({
        bidder: bidder1.publicKey,
        auction: auctionPda,
        bidRecord: bidRecord1,
        bidderBidAta: bidder1BidAta,
        vaultBid: vaultBid,
        bidMint: bidMint,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bidder1])
      .rpc();

    const auctionData = await program.account.auction.fetch(auctionPda);
    expect(auctionData.highestBidder.toBase58()).to.equal(bidder1.publicKey.toBase58());
    expect(auctionData.highestBidAmount.toNumber()).to.equal(250_000_000);
  });

  it("Resolves the auction via crank bot", async () => {
    // Wait for the auction timer to expire
    console.log("Waiting 15 seconds for auction to end...(10 sec delay + 5 sec buffer)");
    await new Promise((resolve) => setTimeout(resolve, 15000));

    winnerNftAta = getAssociatedTokenAddressSync(nftMint, bidder1.publicKey);

    await program.methods.resolveAuction()
      .accountsStrict({
        resolver: crank.publicKey,
        auction: auctionPda,
        winner: bidder1.publicKey,
        maker: maker.publicKey,
        winnerBidRecord: bidRecord1,
        makerBidAta: makerBidAta,
        winnerNftAta: winnerNftAta,
        vaultNft: vaultNft,
        vaultBid: vaultBid,
        nftMint: nftMint,
        bidMint: bidMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([crank])
      .rpc();

    // Check Maker got the money
    const makerBidBalance = (await provider.connection.getTokenAccountBalance(makerBidAta)).value.uiAmount;
    expect(makerBidBalance).to.equal(250); // 250_000_000 / 10^6

    // Check Winner got the NFT
    const winnerNftBalance = (await provider.connection.getTokenAccountBalance(winnerNftAta)).value.uiAmount;
    expect(winnerNftBalance).to.equal(1);

    // Ensure winner's bid record and vault_nft were closed
    const winnerRecordInfo = await provider.connection.getAccountInfo(bidRecord1);
    const vaultNftInfo = await provider.connection.getAccountInfo(vaultNft);
    expect(winnerRecordInfo).to.be.null;
    expect(vaultNftInfo).to.be.null;
  });

  it("Refunds the loser and performs dynamic teardown", async () => {
    await program.methods.claimRefund()
      .accountsStrict({
        bidder: bidder2.publicKey,
        maker: maker.publicKey,
        auction: auctionPda,
        bidRecord: bidRecord2,
        bidderBidAta: bidder2BidAta,
        vaultBid: vaultBid,
        bidMint: bidMint,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([bidder2])
      .rpc();

    // Check Bidder 2 got their money back
    const bidder2Balance = (await provider.connection.getTokenAccountBalance(bidder2BidAta)).value.uiAmount;
    expect(bidder2Balance).to.equal(1000); // Back to starting balance

    // Verify COMPLETE TEARDOWN
    const vaultBidInfo = await provider.connection.getAccountInfo(vaultBid);
    const auctionInfo = await provider.connection.getAccountInfo(auctionPda);
    expect(vaultBidInfo).to.be.null; // Vault closed
    expect(auctionInfo).to.be.null;  // PDA closed manually
  });

  it("Cancels a zero-bidder auction safely", async () => {
    //  Maker mints a new NFT for a new auction
    const newNftMint = await createMint(provider.connection, maker.payer, maker.publicKey, null, 0);
    const newMakerNftAta = getAssociatedTokenAddressSync(newNftMint, maker.publicKey);
    let tx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(maker.publicKey, newMakerNftAta, maker.publicKey, newNftMint)
    );
    await provider.sendAndConfirm(tx);
    await mintTo(provider.connection, maker.payer, newNftMint, newMakerNftAta, maker.publicKey, 1);

    // Make the auction
    const fastEndTime = Math.floor(Date.now() / 1000) + 2;
    const [zeroAuctionPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("auction"), maker.publicKey.toBuffer(), seed2.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    const zeroVaultNft = getAssociatedTokenAddressSync(newNftMint, zeroAuctionPda, true);
    const zeroVaultBid = getAssociatedTokenAddressSync(bidMint, zeroAuctionPda, true);

    await program.methods.makeAuction(seed2, new anchor.BN(fastEndTime), new anchor.BN(1))
      .accountsStrict({
        maker: maker.publicKey,
        nftMint: newNftMint,
        bidMint: bidMint,
        makerNftAta: newMakerNftAta,
        auction: zeroAuctionPda,
        vaultNft: zeroVaultNft,
        vaultBid: zeroVaultBid,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).rpc();

    // Wait for it to expire with zero bids
    console.log("Waiting 4 seconds for zero-bid auction to end...(2 sec delay + 2 sec buffer)");
    await new Promise((resolve) => setTimeout(resolve, 4000));

    // Cancel it
    await program.methods.cancelAuction()
      .accountsStrict({
        maker: maker.publicKey,
        auction: zeroAuctionPda,
        vaultNft: zeroVaultNft,
        makerNftAta: newMakerNftAta,
        nftMint: newNftMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        vaultBid: zeroVaultBid,
        bidMint: bidMint,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      }).rpc();

    // Verify the NFT was returned and the auction PDA was closed
    const returnedBalance = (await provider.connection.getTokenAccountBalance(newMakerNftAta)).value.uiAmount;
    const cancelledAuctionInfo = await provider.connection.getAccountInfo(zeroAuctionPda);

    expect(returnedBalance).to.equal(1);
    expect(cancelledAuctionInfo).to.be.null;
  });
});