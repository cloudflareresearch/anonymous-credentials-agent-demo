import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card'
import { Button } from './components/ui/button'
import { Input } from './components/ui/input'
import { Label } from './components/ui/label'
import { Alert, AlertDescription } from './components/ui/alert'
import { Badge } from './components/ui/badge'
import { Separator } from './components/ui/separator'
import { ImageWithFallback } from './components/figma/ImageWithFallback'
import { Shield, CreditCard, AlertCircle, CheckCircle, User, HelpCircle, RotateCcw, Link } from 'lucide-react'

import init, { request_credits, finalize_credits, spend_tokens, update_refund } from './wasm/act_client_demo';

const DEFAULT_ISSUER = 'https://act-issuer-demo.cloudflareresearch.com/';

class MyState {
  constructor(
    public credits = 0,
    public publicKey?: Uint8Array,
    public creditToken?: Uint8Array,
  ) { }
}

let myState = new MyState()

export default function App() {
  const [currentCredits, setCurrentCredits] = useState(0)
  const [requestAmount, setRequestAmount] = useState('')
  const [spendAmount, setSpendAmount] = useState('')
  const [issuerUrl, setIssuerUrl] = useState(DEFAULT_ISSUER)
  const [requestError, setRequestError] = useState('')
  const [spendError, setSpendError] = useState('')
  const [urlError, setUrlError] = useState('')
  const [lastTransaction, setLastTransaction] = useState<{
    type: 'request' | 'spend'
    credits: number
    success: boolean
    message: string
  } | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    // Initialize the Wasm module
    const loadAndUseWasm = async () => { await init(); };
    loadAndUseWasm();
  }, []);

  const url = (endpoint: string) => { return new URL(endpoint, issuerUrl) }

  const validateInput = (value: string): boolean => {
    const num = Number(value)
    return !isNaN(num) && Number.isInteger(num) && num > 0 && num < 256
  }

  const validateUrl = (url: string): boolean => {
    try {
      const urlObj = new URL(url)
      return urlObj.protocol === 'http:' || urlObj.protocol === 'https:'
    } catch {
      return false
    }
  }

  const handleRestart = () => {
    myState = new MyState()
    setCurrentCredits(myState.credits)
    setRequestAmount('')
    setSpendAmount('')
    setIsLoading(false)
    setIssuerUrl(DEFAULT_ISSUER)
    setRequestError('')
    setSpendError('')
    setUrlError('')
    setLastTransaction(null)
  }

  const fetchPublicKey = async (): Promise<boolean> => {
    const reqPublic = await fetch(url("/public"));
    if (reqPublic.status != 200) {
      setUrlError('cannot fetch public key')
      return false
    }

    myState.publicKey = await reqPublic.bytes();
    return true
  }

  const handleRequestCredits = async () => {
    setRequestError('')
    setUrlError('')

    if (!requestAmount.trim()) {
      setRequestError('Please enter an amount.')
      return
    }

    if (!validateInput(requestAmount)) {
      setRequestError('Please enter an integer between 1 and 255.')
      return
    }

    // Use default URL if none provided
    const urlToUse = issuerUrl.trim() || DEFAULT_ISSUER

    if (!validateUrl(urlToUse)) {
      setRequestError('Please enter a valid URL (http:// or https://)')
      return
    }

    try {
      setIsLoading(true)

      if (!await fetchPublicKey() || !myState.publicKey) {
        setRequestError('Cannot retrieve issuer\'s public key')
        setLastTransaction({
          type: 'request',
          credits: 0,
          success: false,
          message: `Failure to request credits`
        })
        setIsLoading(false)
        return
      }

      const credits = parseInt(requestAmount)
      const preReq = request_credits();

      const credReqCbor = preReq.issuance_request;
      let body = new Uint8Array(1 + credReqCbor.length);
      body.set([credits], 0)
      body.set(credReqCbor, 1)
      const resCred = await fetch(url("/request"), { method: 'POST', body: body, });
      switch (resCred.status) {
        case 200:
          const credResCbor = await resCred.bytes();
          const credit_token = finalize_credits(myState.publicKey, preReq, credResCbor);
          myState.credits = credits;
          myState.creditToken = credit_token;
          setCurrentCredits(myState.credits);
          setLastTransaction({
            type: 'request',
            credits,
            success: true,
            message: `Successfully received ${credits} credits.`
          })
          setRequestAmount('')
          break;
        case 400:
          setRequestError("Issuer error: invalid response of credential.")
          setLastTransaction({
            type: 'request',
            credits,
            success: false,
            message: `Failure to request credits.`
          })
          break;
      }

      setIsLoading(false)
    } catch (e) {
      setLastTransaction({
        type: 'request',
        credits: 0,
        success: false,
        message: `Failure to request credits: ${(e as Error).message}.`
      })
      setIsLoading(false)
    }
  }

  const handleSpendCredits = async () => {
    setSpendError('')

    if (!spendAmount.trim()) {
      setSpendError('Please enter an amount')
      return
    }

    if (!validateInput(spendAmount)) {
      setSpendError('Please enter a valid positive integer.')
      return
    }

    if (!myState.creditToken) {
      setSpendError('Invalid credit state')
      return
    }

    if (!myState.publicKey) {
      setSpendError('Invalid public key')
      return
    }

    const spendCredits = parseInt(spendAmount)
    if (spendCredits > currentCredits) {
      setSpendError('Insufficient credits, but still trying')
    }

    setIsLoading(true)

    const preSpend = spend_tokens(spendCredits, myState.creditToken);
    const resSpend = await fetch(url("/spend"), { method: 'POST', body: preSpend.spend_proof as any });
    switch (resSpend.status) {
      case 200:
        const refund = await resSpend.bytes();
        const credit_token = update_refund(preSpend, refund, myState.publicKey);
        myState.creditToken = credit_token
        myState.credits -= spendCredits
        setCurrentCredits(myState.credits)
        setLastTransaction({
          type: 'spend',
          credits: spendCredits,
          success: true,
          message: `Successfully spent ${spendCredits} credits`
        })

        break;
      case 400:
        setSpendError(`Server error: ${await resSpend.text()}.`)
        setLastTransaction({
          type: 'spend',
          credits: spendCredits,
          success: false,
          message: 'Transaction failed. Please try again.'
        })
        if (myState.credits == 0) {
          setSpendError(prev => prev + " Issuer error: Try to obtain more tokens.")
        }
        break;
      default:
        setSpendError(`Issuer error with code ${resSpend.status}`)
    }

    setIsLoading(false)
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Banner */}
      <div className="relative h-64 w-full overflow-hidden">
        <ImageWithFallback
          src="https://images.unsplash.com/photo-1605101479435-005f9c563944?q=80&w=1470&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D"
          alt="ACT Demo"
          className="w-full h-full object-cover"
        />
        <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
          <div className="text-center text-white space-y-2">
            <div className="flex items-center justify-center gap-2 mb-4">
              <Shield className="h-12 w-12" />
            </div>
            <h1 className="text-3xl font-medium">🦊 Demo: ACT Anonymous Credit Tokens</h1>
            <p className="text-lg opacity-90">A privacy-preserving authentication protocol that enables numerical credit systems without tracking individual clients</p>
          </div>
        </div>
      </div>

      <div className="container mx-auto px-4 py-8 max-w-4xl">
        {/* How it works section */}
        <Card className="mb-8">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CreditCard className="h-5 w-5" />
              How ACT Works
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p>
              ACT credit system allows you to efficiently request digital credits to a server and redeem credits without identifying the user.
              It is a simple two-step process:
            </p>
            <div className="grid md:grid-cols-2 gap-6">
              <div className="space-y-2">
                <h3 className="font-medium">1. Request Credits</h3>
                <p className="text-muted-foreground">
                  Enter the amount of credits you need and submit your request. The system will process your request and add credits to your balance.
                </p>
              </div>
              <div className="space-y-2">
                <h3 className="font-medium">2. Spend Credits</h3>
                <p className="text-muted-foreground">
                  Use your available credits for accessing to a resource, an origin, or an API.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Current balance and restart button */}
        <Card className="mb-8">
          <CardHeader>
            <CardTitle>Current Balance</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <Badge variant="secondary" className="text-lg px-4 py-2">
                {currentCredits} Credits
              </Badge>
              <Button
                variant="outline"
                onClick={handleRestart}
                className="flex items-center gap-2"
                disabled={isLoading}
              >
                <RotateCcw className="h-4 w-4" />
                Restart Process
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Transaction status */}
        {lastTransaction && (
          <Alert className={`mb-8 ${lastTransaction.success ? 'border-green-200' : 'border-red-200'}`}>
            <div className="flex items-center gap-2">
              {lastTransaction.success ? (
                <CheckCircle className="h-4 w-4 text-green-600" />
              ) : (
                <AlertCircle className="h-4 w-4 text-red-600" />
              )}
              <AlertDescription className={lastTransaction.success ? 'text-green-700' : 'text-red-700'}>
                {lastTransaction.message}
              </AlertDescription>
            </div>
          </Alert>
        )}

        <div className="grid md:grid-cols-2 gap-8 mb-8">
          {/* Request Credits */}
          <Card>
            <CardHeader>
              <CardTitle>Request Credits</CardTitle>
              <CardDescription>
                Enter the amount of credits you'd like to request
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="request-amount">Amount</Label>
                <Input
                  id="request-amount"
                  type="text"
                  placeholder="Enter number of credits"
                  value={requestAmount}
                  onChange={(e) => {
                    setRequestAmount(e.target.value)
                    setRequestError('')
                  }}
                  className={requestError ? 'border-red-500' : ''}
                />
                {requestError && (
                  <p className="text-sm text-red-600 flex items-center gap-1">
                    <AlertCircle className="h-3 w-3" />
                    {requestError}
                  </p>
                )}
              </div>
              <Button
                onClick={handleRequestCredits}
                className="w-full"
                disabled={isLoading}
              >
                {isLoading ? 'Processing...' : 'Request Credits'}
              </Button>
            </CardContent>
          </Card>

          {/* Spend Credits */}
          <Card>
            <CardHeader>
              <CardTitle>Spend Credits</CardTitle>
              <CardDescription>
                Enter the amount of credits you'd like to spend
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="spend-amount">Amount</Label>
                <Input
                  id="spend-amount"
                  type="text"
                  placeholder="Enter number of credits"
                  value={spendAmount}
                  onChange={(e) => {
                    setSpendAmount(e.target.value)
                    setSpendError('')
                  }}
                  className={spendError ? 'border-red-500' : ''}
                />
                {spendError && (
                  <p className="text-sm text-red-600 flex items-center gap-1">
                    <AlertCircle className="h-3 w-3" />
                    {spendError}
                  </p>
                )}
              </div>
              <Button
                onClick={handleSpendCredits}
                className="w-full"
                disabled={isLoading}
              >
                {isLoading ? 'Processing...' : 'Spend Credits'}
              </Button>
            </CardContent>
          </Card>
        </div>

        <Separator className="my-8" />

        {/* Server URL Configuration - Optional */}
        <Card className="mb-8">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Link className="h-5 w-5" />
              Issuer Configuration (Optional)
            </CardTitle>
            <CardDescription>
              Enter the Issuer URL: (e.g. {DEFAULT_ISSUER})
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="server-url">Issuer URL</Label>
              <Input
                id="server-url"
                placeholder={DEFAULT_ISSUER}
                value={issuerUrl}
                onChange={(e) => {
                  setIssuerUrl(e.target.value)
                  setUrlError('')
                }}
                className={urlError ? 'border-red-500' : ''}
              />
              {urlError && (
                <p className="text-sm text-red-600 flex items-center gap-1">
                  <AlertCircle className="h-3 w-3" />
                  {urlError}
                </p>
              )}
            </div>
          </CardContent>
        </Card>

        <Separator className="my-8" />

        {/* Author and FAQ */}
        <div className="grid md:grid-cols-2 gap-8">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <User className="h-5 w-5" />
                About this Demo
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p>
                This simple website shows how a credit system can be used in a browser environment.
                The same functionality can be applied to a service fetching resources from an API endpoint.
              </p>
              <p>
                Cloudflare Research team is currently investigating about anonymous credential systems.
              </p>
              <div className="space-y-2">
                <p className="font-medium">Contact Information:</p>
                <p className="text-muted-foreground"><a href="mailto:ask-research@cloudflare.com">ask-research@cloudflare.com</a></p>
                <p className="text-muted-foreground">Version 0.1.0</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <HelpCircle className="h-5 w-5" />
                Frequently Asked Questions
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-3">
                <div>
                  <p className="font-medium">What is the main use case?</p>
                  <p className="text-sm text-muted-foreground">Enables to rate limit users trying to access a resource.</p>
                </div>
                <div>
                  <p className="font-medium">Does the user's identity gets protected?</p>
                  <p className="text-sm text-muted-foreground">Yes, server cannot determine which user is spending credits.</p>
                </div>
                <div>
                  <p className="font-medium">What happens when I restart the process?</p>
                  <p className="text-sm text-muted-foreground">The restart button clears all credits and forms to start fresh.</p>
                </div>
                <div>
                  <p className="font-medium">How does the two-step process work?</p>
                  <p className="text-sm text-muted-foreground">Simply request credits first, then spend them as needed. The process is streamlined for efficiency.</p>
                </div>
                <div>
                  <p className="font-medium">I want to know more or to contribute.</p>
                  <p className="text-sm text-muted-foreground">Reach the Cloudflare Research Team.</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
