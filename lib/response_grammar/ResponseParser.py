# Generated from Response.g4 by ANTLR 4.7.2
# encoding: utf-8
from antlr4 import *
from io import StringIO
from typing.io import TextIO
import sys


def serializedATN():
    with StringIO() as buf:
        buf.write("\3\u608b\ua72a\u8133\ub9ed\u417c\u3be7\u7786\u5964\3\20")
        buf.write(")\4\2\t\2\4\3\t\3\4\4\t\4\4\5\t\5\3\2\3\2\6\2\r\n\2\r")
        buf.write("\2\16\2\16\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\3\5\3")
        buf.write("\33\n\3\3\4\3\4\6\4\37\n\4\r\4\16\4 \3\4\3\4\3\4\3\5\3")
        buf.write("\5\3\5\3\5\2\2\6\2\4\6\b\2\2\2\60\2\f\3\2\2\2\4\32\3\2")
        buf.write("\2\2\6\34\3\2\2\2\b%\3\2\2\2\n\r\5\4\3\2\13\r\7\6\2\2")
        buf.write("\f\n\3\2\2\2\f\13\3\2\2\2\r\16\3\2\2\2\16\f\3\2\2\2\16")
        buf.write("\17\3\2\2\2\17\3\3\2\2\2\20\33\5\6\4\2\21\33\7\7\2\2\22")
        buf.write("\33\7\b\2\2\23\33\7\t\2\2\24\33\7\n\2\2\25\33\7\13\2\2")
        buf.write("\26\33\7\f\2\2\27\33\7\r\2\2\30\33\7\16\2\2\31\33\7\17")
        buf.write("\2\2\32\20\3\2\2\2\32\21\3\2\2\2\32\22\3\2\2\2\32\23\3")
        buf.write("\2\2\2\32\24\3\2\2\2\32\25\3\2\2\2\32\26\3\2\2\2\32\27")
        buf.write("\3\2\2\2\32\30\3\2\2\2\32\31\3\2\2\2\33\5\3\2\2\2\34\36")
        buf.write("\7\3\2\2\35\37\5\b\5\2\36\35\3\2\2\2\37 \3\2\2\2 \36\3")
        buf.write("\2\2\2 !\3\2\2\2!\"\3\2\2\2\"#\5\2\2\2#$\7\4\2\2$\7\3")
        buf.write("\2\2\2%&\5\2\2\2&\'\7\5\2\2\'\t\3\2\2\2\6\f\16\32 ")
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
    RULE_respObj = 1
    RULE_respList = 2
    RULE_listElement = 3

    ruleNames =  [ "response", "respObj", "respList", "listElement" ]

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

        def respObj(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(ResponseParser.RespObjContext)
            else:
                return self.getTypedRuleContext(ResponseParser.RespObjContext,i)


        def STRING(self, i:int=None):
            if i is None:
                return self.getTokens(ResponseParser.STRING)
            else:
                return self.getToken(ResponseParser.STRING, i)

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
            self.state = 10 
            self._errHandler.sync(self)
            _la = self._input.LA(1)
            while True:
                self.state = 10
                self._errHandler.sync(self)
                token = self._input.LA(1)
                if token in [ResponseParser.T__0, ResponseParser.REACT, ResponseParser.NOUN, ResponseParser.ADJ, ResponseParser.ADV, ResponseParser.OWL, ResponseParser.COUNT, ResponseParser.MEMBER, ResponseParser.AUTHOR, ResponseParser.CAPTURE]:
                    self.state = 8
                    self.respObj()
                    pass
                elif token in [ResponseParser.STRING]:
                    self.state = 9
                    self.match(ResponseParser.STRING)
                    pass
                else:
                    raise NoViableAltException(self)

                self.state = 12 
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


    class RespObjContext(ParserRuleContext):

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def respList(self):
            return self.getTypedRuleContext(ResponseParser.RespListContext,0)


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
            return ResponseParser.RULE_respObj

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterRespObj" ):
                listener.enterRespObj(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitRespObj" ):
                listener.exitRespObj(self)




    def respObj(self):

        localctx = ResponseParser.RespObjContext(self, self._ctx, self.state)
        self.enterRule(localctx, 2, self.RULE_respObj)
        try:
            self.state = 24
            self._errHandler.sync(self)
            token = self._input.LA(1)
            if token in [ResponseParser.T__0]:
                self.enterOuterAlt(localctx, 1)
                self.state = 14
                self.respList()
                pass
            elif token in [ResponseParser.REACT]:
                self.enterOuterAlt(localctx, 2)
                self.state = 15
                self.match(ResponseParser.REACT)
                pass
            elif token in [ResponseParser.NOUN]:
                self.enterOuterAlt(localctx, 3)
                self.state = 16
                self.match(ResponseParser.NOUN)
                pass
            elif token in [ResponseParser.ADJ]:
                self.enterOuterAlt(localctx, 4)
                self.state = 17
                self.match(ResponseParser.ADJ)
                pass
            elif token in [ResponseParser.ADV]:
                self.enterOuterAlt(localctx, 5)
                self.state = 18
                self.match(ResponseParser.ADV)
                pass
            elif token in [ResponseParser.OWL]:
                self.enterOuterAlt(localctx, 6)
                self.state = 19
                self.match(ResponseParser.OWL)
                pass
            elif token in [ResponseParser.COUNT]:
                self.enterOuterAlt(localctx, 7)
                self.state = 20
                self.match(ResponseParser.COUNT)
                pass
            elif token in [ResponseParser.MEMBER]:
                self.enterOuterAlt(localctx, 8)
                self.state = 21
                self.match(ResponseParser.MEMBER)
                pass
            elif token in [ResponseParser.AUTHOR]:
                self.enterOuterAlt(localctx, 9)
                self.state = 22
                self.match(ResponseParser.AUTHOR)
                pass
            elif token in [ResponseParser.CAPTURE]:
                self.enterOuterAlt(localctx, 10)
                self.state = 23
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


    class RespListContext(ParserRuleContext):

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
            return ResponseParser.RULE_respList

        def enterRule(self, listener:ParseTreeListener):
            if hasattr( listener, "enterRespList" ):
                listener.enterRespList(self)

        def exitRule(self, listener:ParseTreeListener):
            if hasattr( listener, "exitRespList" ):
                listener.exitRespList(self)




    def respList(self):

        localctx = ResponseParser.RespListContext(self, self._ctx, self.state)
        self.enterRule(localctx, 4, self.RULE_respList)
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 26
            self.match(ResponseParser.T__0)
            self.state = 28 
            self._errHandler.sync(self)
            _alt = 1
            while _alt!=2 and _alt!=ATN.INVALID_ALT_NUMBER:
                if _alt == 1:
                    self.state = 27
                    self.listElement()

                else:
                    raise NoViableAltException(self)
                self.state = 30 
                self._errHandler.sync(self)
                _alt = self._interp.adaptivePredict(self._input,3,self._ctx)

            self.state = 32
            self.response()
            self.state = 33
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
            self.state = 35
            self.response()
            self.state = 36
            self.match(ResponseParser.T__2)
        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx





