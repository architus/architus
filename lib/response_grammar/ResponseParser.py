# Generated from Response.g4 by ANTLR 4.7.2
# encoding: utf-8
from antlr4 import *
from io import StringIO
from typing.io import TextIO
import sys


def serializedATN():
    with StringIO() as buf:
        buf.write("\3\u608b\ua72a\u8133\ub9ed\u417c\u3be7\u7786\u5964\3\20")
        buf.write("+\4\2\t\2\4\3\t\3\4\4\t\4\4\5\t\5\3\2\5\2\f\n\2\3\2\6")
        buf.write("\2\17\n\2\r\2\16\2\20\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3")
        buf.write("\3\3\3\3\5\3\35\n\3\3\4\3\4\6\4!\n\4\r\4\16\4\"\3\4\3")
        buf.write("\4\3\4\3\5\3\5\3\5\3\5\2\2\6\2\4\6\b\2\2\2\62\2\16\3\2")
        buf.write("\2\2\4\34\3\2\2\2\6\36\3\2\2\2\b\'\3\2\2\2\n\f\5\4\3\2")
        buf.write("\13\n\3\2\2\2\13\f\3\2\2\2\f\r\3\2\2\2\r\17\7\6\2\2\16")
        buf.write("\13\3\2\2\2\17\20\3\2\2\2\20\16\3\2\2\2\20\21\3\2\2\2")
        buf.write("\21\3\3\2\2\2\22\35\5\6\4\2\23\35\7\7\2\2\24\35\7\b\2")
        buf.write("\2\25\35\7\t\2\2\26\35\7\n\2\2\27\35\7\13\2\2\30\35\7")
        buf.write("\f\2\2\31\35\7\r\2\2\32\35\7\16\2\2\33\35\7\17\2\2\34")
        buf.write("\22\3\2\2\2\34\23\3\2\2\2\34\24\3\2\2\2\34\25\3\2\2\2")
        buf.write("\34\26\3\2\2\2\34\27\3\2\2\2\34\30\3\2\2\2\34\31\3\2\2")
        buf.write("\2\34\32\3\2\2\2\34\33\3\2\2\2\35\5\3\2\2\2\36 \7\3\2")
        buf.write("\2\37!\5\b\5\2 \37\3\2\2\2!\"\3\2\2\2\" \3\2\2\2\"#\3")
        buf.write("\2\2\2#$\3\2\2\2$%\5\2\2\2%&\7\4\2\2&\7\3\2\2\2\'(\5\2")
        buf.write("\2\2()\7\5\2\2)\t\3\2\2\2\6\13\20\34\"")
        return buf.getvalue()


class ResponseParser ( Parser ):

    grammarFileName = "Response.g4"

    atn = ATNDeserializer().deserialize(serializedATN())

    decisionsToDFA = [ DFA(ds, i) for i, ds in enumerate(atn.decisionToState) ]

    sharedContextCache = PredictionContextCache()

    literalNames = [ "<INVALID>", "'['", "']'", "','" ]

    symbolicNames = [ "<INVALID>", "<INVALID>", "<INVALID>", "<INVALID>", 
                      "STRING", "REACT", "NOUN", "ADJ", "ADV", "OWL", "COUNT", 
                      "MEMBER", "AUTHOR", "CAPTURE", "WS" ]

    RULE_response = 0
    RULE_responseObj = 1
    RULE_architusList = 2
    RULE_listElement = 3

    ruleNames =  [ "response", "responseObj", "architusList", "listElement" ]

    EOF = Token.EOF
    T__0=1
    T__1=2
    T__2=3
    STRING=4
    REACT=5
    NOUN=6
    ADJ=7
    ADV=8
    OWL=9
    COUNT=10
    MEMBER=11
    AUTHOR=12
    CAPTURE=13
    WS=14

    def __init__(self, input:TokenStream, output:TextIO = sys.stdout):
        super().__init__(input, output)
        self.checkVersion("4.7.2")
        self._interp = ParserATNSimulator(self, self.atn, self.decisionsToDFA, self.sharedContextCache)
        self._predicates = None




    class ResponseContext(ParserRuleContext):

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def STRING(self, i:int=None):
            if i is None:
                return self.getTokens(ResponseParser.STRING)
            else:
                return self.getToken(ResponseParser.STRING, i)

        def responseObj(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(ResponseParser.ResponseObjContext)
            else:
                return self.getTypedRuleContext(ResponseParser.ResponseObjContext,i)


        def getRuleIndex(self):
            return ResponseParser.RULE_response

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterResponse" ):
                listener.enterResponse(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitResponse" ):
                listener.exitResponse(self)




    def response(self):

        localctx = ResponseParser.ResponseContext(self, self._ctx, self.state)
        self.enterRule(localctx, 0, self.RULE_response)
        self._la = 0 # Token type
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 12 
            self._errHandler.sync(self)
            _la = self._input.LA(1)
            while True:
                self.state = 9
                self._errHandler.sync(self)
                _la = self._input.LA(1)
                if (((_la) & ~0x3f) == 0 and ((1 << _la) & ((1 << ResponseParser.T__0) | (1 << ResponseParser.REACT) | (1 << ResponseParser.NOUN) | (1 << ResponseParser.ADJ) | (1 << ResponseParser.ADV) | (1 << ResponseParser.OWL) | (1 << ResponseParser.COUNT) | (1 << ResponseParser.MEMBER) | (1 << ResponseParser.AUTHOR) | (1 << ResponseParser.CAPTURE))) != 0):
                    self.state = 8
                    self.responseObj()


                self.state = 11
                self.match(ResponseParser.STRING)
                self.state = 14 
                self._errHandler.sync(self)
                _la = self._input.LA(1)
                if not ((((_la) & ~0x3f) == 0 and ((1 << _la) & ((1 << ResponseParser.T__0) | (1 << ResponseParser.STRING) | (1 << ResponseParser.REACT) | (1 << ResponseParser.NOUN) | (1 << ResponseParser.ADJ) | (1 << ResponseParser.ADV) | (1 << ResponseParser.OWL) | (1 << ResponseParser.COUNT) | (1 << ResponseParser.MEMBER) | (1 << ResponseParser.AUTHOR) | (1 << ResponseParser.CAPTURE))) != 0)):
                    break

        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx


    class ResponseObjContext(ParserRuleContext):

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def architusList(self):
            return self.getTypedRuleContext(ResponseParser.ArchitusListContext,0)


        def REACT(self):
            return self.getToken(ResponseParser.REACT, 0)

        def NOUN(self):
            return self.getToken(ResponseParser.NOUN, 0)

        def ADJ(self):
            return self.getToken(ResponseParser.ADJ, 0)

        def ADV(self):
            return self.getToken(ResponseParser.ADV, 0)

        def OWL(self):
            return self.getToken(ResponseParser.OWL, 0)

        def COUNT(self):
            return self.getToken(ResponseParser.COUNT, 0)

        def MEMBER(self):
            return self.getToken(ResponseParser.MEMBER, 0)

        def AUTHOR(self):
            return self.getToken(ResponseParser.AUTHOR, 0)

        def CAPTURE(self):
            return self.getToken(ResponseParser.CAPTURE, 0)

        def getRuleIndex(self):
            return ResponseParser.RULE_responseObj

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterResponseObj" ):
                listener.enterResponseObj(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitResponseObj" ):
                listener.exitResponseObj(self)




    def responseObj(self):

        localctx = ResponseParser.ResponseObjContext(self, self._ctx, self.state)
        self.enterRule(localctx, 2, self.RULE_responseObj)
        try:
            self.state = 26
            self._errHandler.sync(self)
            token = self._input.LA(1)
            if token in [ResponseParser.T__0]:
                self.enterOuterAlt(localctx, 1)
                self.state = 16
                self.architusList()
                pass
            elif token in [ResponseParser.REACT]:
                self.enterOuterAlt(localctx, 2)
                self.state = 17
                self.match(ResponseParser.REACT)
                pass
            elif token in [ResponseParser.NOUN]:
                self.enterOuterAlt(localctx, 3)
                self.state = 18
                self.match(ResponseParser.NOUN)
                pass
            elif token in [ResponseParser.ADJ]:
                self.enterOuterAlt(localctx, 4)
                self.state = 19
                self.match(ResponseParser.ADJ)
                pass
            elif token in [ResponseParser.ADV]:
                self.enterOuterAlt(localctx, 5)
                self.state = 20
                self.match(ResponseParser.ADV)
                pass
            elif token in [ResponseParser.OWL]:
                self.enterOuterAlt(localctx, 6)
                self.state = 21
                self.match(ResponseParser.OWL)
                pass
            elif token in [ResponseParser.COUNT]:
                self.enterOuterAlt(localctx, 7)
                self.state = 22
                self.match(ResponseParser.COUNT)
                pass
            elif token in [ResponseParser.MEMBER]:
                self.enterOuterAlt(localctx, 8)
                self.state = 23
                self.match(ResponseParser.MEMBER)
                pass
            elif token in [ResponseParser.AUTHOR]:
                self.enterOuterAlt(localctx, 9)
                self.state = 24
                self.match(ResponseParser.AUTHOR)
                pass
            elif token in [ResponseParser.CAPTURE]:
                self.enterOuterAlt(localctx, 10)
                self.state = 25
                self.match(ResponseParser.CAPTURE)
                pass
            else:
                raise NoViableAltException(self)

        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx


    class ArchitusListContext(ParserRuleContext):

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def response(self):
            return self.getTypedRuleContext(ResponseParser.ResponseContext,0)


        def listElement(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(ResponseParser.ListElementContext)
            else:
                return self.getTypedRuleContext(ResponseParser.ListElementContext,i)


        def getRuleIndex(self):
            return ResponseParser.RULE_architusList

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterArchitusList" ):
                listener.enterArchitusList(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitArchitusList" ):
                listener.exitArchitusList(self)




    def architusList(self):

        localctx = ResponseParser.ArchitusListContext(self, self._ctx, self.state)
        self.enterRule(localctx, 4, self.RULE_architusList)
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 28
            self.match(ResponseParser.T__0)
            self.state = 30 
            self._errHandler.sync(self)
            _alt = 1
            while _alt!=2 and _alt!=ATN.INVALID_ALT_NUMBER:
                if _alt == 1:
                    self.state = 29
                    self.listElement()

                else:
                    raise NoViableAltException(self)
                self.state = 32 
                self._errHandler.sync(self)
                _alt = self._interp.adaptivePredict(self._input,3,self._ctx)

            self.state = 34
            self.response()
            self.state = 35
            self.match(ResponseParser.T__1)
        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx


    class ListElementContext(ParserRuleContext):

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def response(self):
            return self.getTypedRuleContext(ResponseParser.ResponseContext,0)


        def getRuleIndex(self):
            return ResponseParser.RULE_listElement

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterListElement" ):
                listener.enterListElement(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitListElement" ):
                listener.exitListElement(self)




    def listElement(self):

        localctx = ResponseParser.ListElementContext(self, self._ctx, self.state)
        self.enterRule(localctx, 6, self.RULE_listElement)
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 37
            self.response()
            self.state = 38
            self.match(ResponseParser.T__2)
        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx





