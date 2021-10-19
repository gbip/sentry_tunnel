CHANGELOG
=========

1.0.7		(XXXX-XX-XX)
-----------------------

* Allow multiple sentry relays in configuration

1.0.6		(2021-10-19)
-----------------------

* check dsn hostname against the config
* add 'sentry_key' parameter to handle relay version before 21.6

1.0.5		(2021-10-19)
-----------------------

* Fix error propagation to original client

1.0.4		(2021-10-19)
------------------------

* Propagate http response from sentry to the original client

1.0.3		(2021-10-19)
------------------------

* Fix server response mime type
* Fix sentry forward url

1.0.2		(2021-10-19)
------------------------

* Fix missing ssl cert in docker image

1.0.1		(2021-10-19)
------------------------

* Fix content length verification set to 1 kb. Uses 1 mb now.

1.0.0		(2021-10-19)
------------------------

First release !
