import json

import stripe
from flask import request

from src.util import CustomResource
from lib.config import logger, stripe_secret_key, stripe_webhook_secret


stripe.api_key = stripe_secret_key

intent = stripe.PaymentIntent.create(
    amount=1099,
    currency='usd',
    # Verify your integration in this guide by including this parameter
    metadata={'integration_check': 'accept_a_payment'},
)


class Checkout(CustomResource):
    def post(self):
        session = stripe.checkout.Session.create(
            payment_method_types=['card'],
            line_items=[{
                'price': 'price_1IqqGiHUEjNeJTDOoSGqvp28',
                'quantity': 1,
            }],
            mode='subscription',
            success_url='https://example.com/success?session_id={CHECKOUT_SESSION_ID}',
            cancel_url='https://example.com/cancel',
        )
        return json.dumps({'id': session.id})


class Stripe(CustomResource):
    def post(self):
        # Retrieve the event by verifying the signature using the raw body and secret if webhook signing is configured.
        signature = request.headers.get('stripe-signature')
        try:
            event = stripe.Webhook.construct_event(
                payload=request.data, sig_header=signature, secret=stripe_webhook_secret)
            data = event['data']
        except Exception as e:
            return e
        # Get the type of webhook event sent - used to check the status of PaymentIntents.
        event_type = event['type']

        if event_type == 'checkout.session.completed':
            # Payment is successful and the subscription is created.
            # You should provision the subscription and save the customer ID to your database.
            print(data)
        elif event_type == 'invoice.paid':
            # Continue to provision the subscription as payments continue to be made.
            # Store the status in your database and check when a user accesses your service.
            # This approach helps you avoid hitting rate limits.
            print(data)
        elif event_type == 'invoice.payment_failed':
            # The payment failed or the customer does not have a valid payment method.
            # The subscription becomes past_due. Notify your customer and send them to the
            # customer portal to update their payment information.
            print(data)
        else:
            print('Unhandled event type {}'.format(event_type))

        return {'status': 'success'}
